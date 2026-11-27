<!--
  The hub as hardware (dark in every theme; only the canvas around it changes). Modules are
  placed by their socket on the Base and rotated like the real thing; keys show the hub's own
  pictures while the edited profile is live, and previews from the config otherwise.
  Markers (pinned, integration, inherited) sit on the corner of a control, outside its screen;
  a claimed module gets a dashed outline with a tag. Tab enters the hub, arrows move between controls.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import { currentPage, currentProfile, displayKey, editor, flashKey, hub, isEditingLive, ledColor } from '../../lib/hub.svelte';
  import {
    DIAL_PX, FOOTPRINT, KNOB_PX, ROLLER_PX, U, boundsPx, cssMatrix, faderBox, keyBox, layoutModules, type Box,
  } from '../../lib/geometry';
  import { controlIn, ownerOf, profileByName, type ControlRef, type Owner } from '../../lib/model';
  import { previewOf } from '../../lib/previews.svelte';
  import { TARGET_DIAL, type Action, type ClaimInfo, type Face, type ModuleKind, type ModuleState, type Selection } from '../../lib/types';
  import CanvasMarker, { type MarkerKind } from '../ui/CanvasMarker.svelte';
  import ClaimOutline from '../ui/ClaimOutline.svelte';
  import { ctlId, neighbour, sameSel, shortAction, type Dir, type Spot } from './controls';

  const PAD = 44;
  const MAX_SCALE = 1.2;

  type Cfg = Face & { cw?: Action; ccw?: Action; press?: Action; away?: Action; toward?: Action; change?: Action };

  interface Ctl {
    id: string;
    ref: ControlRef;
    serial: string;
    kind: ModuleKind;
    /** Natural px box in its module. */
    box: Box;
    /** World px bounds. */
    bounds: Box;
    flash: string;
    /** Rotation of the module on screen, degrees: previews are drawn upright, the hub's own pictures are pre-rotated. */
    rot: number;
  }

  const live = $derived(isEditingLive());
  const profile = $derived(currentProfile());
  const pageName = $derived(currentPage()?.name);
  const ctx = $derived({ profile: profile?.name ?? '', page: pageName, setup: editor.setup || undefined, claims: live ? hub.claims : [] });

  // ─── layout: where every module sits ───
  const layout = $derived(layoutModules(hub.modules, hub.state.rotation ?? 90, PAD));

  const controls = $derived.by(() => {
    const out: Ctl[] = [];
    for (const { m, matrix } of layout.placed) {
      const add = (ref: ControlRef, box: Box, flashControl: string, flashIndex: number) =>
        out.push({ id: ctlId(ref), ref, serial: m.serial, kind: m.kind, box, bounds: boundsPx(matrix, box), flash: flashKey(m.serial, flashControl, flashIndex), rot: Math.round((Math.atan2(matrix[1], matrix[0]) * 180) / Math.PI) });
      if (m.kind === 'keypad') for (let i = 0; i < 15; i++) add({ kind: 'key', index: i }, keyBox(i), 'key', i);
      else if (m.kind === 'dial') add({ kind: 'dial' }, [DIAL_PX.cx - DIAL_PX.ring, DIAL_PX.cy - DIAL_PX.ring, DIAL_PX.ring * 2, DIAL_PX.ring * 2], 'dial', 0);
      else if (m.kind === 'knob') KNOB_PX.centers.forEach(([cx, cy], i) => add({ kind: 'knob', index: i }, [cx - KNOB_PX.r, cy - KNOB_PX.r, KNOB_PX.r * 2, KNOB_PX.r * 2], 'knob', i));
      else if (m.kind === 'roller') (['left', 'right'] as const).forEach((side, i) => add({ kind: 'roller', side }, ROLLER_PX[side], 'roller', i));
      else if (m.kind === 'fader') for (let i = 0; i < 5; i++) add({ kind: 'fader', index: i }, faderBox(i), 'fader', i);
    }
    return out;
  });
  const byModule = $derived.by(() => {
    const map = new Map<string, Ctl[]>();
    for (const c of controls) map.set(c.serial, [...(map.get(c.serial) ?? []), c]);
    return map;
  });

  // ─── what each control shows and who owns it ───
  function resolve(ref: ControlRef): { owner: Owner; cfg: Cfg | undefined } {
    const c = hub.config;
    const owner = ownerOf(c, ctx, ref, live ? hub.state.reason : undefined);
    const self = profileByName(c, ctx.profile);
    let cfg: unknown;
    if (owner.kind === 'pinned') cfg = owner.pinned;
    else if (owner.kind === 'profile') cfg = self && controlIn(self, ref, pageName);
    else if (owner.kind === 'base') {
      const base = profileByName(c, owner.profile);
      cfg = base && controlIn(base, ref, pageName, self);
    }
    return { owner, cfg: cfg as Cfg | undefined };
  }
  const info = $derived(new Map(controls.map((c) => [c.id, resolve(c.ref)])));

  /** The picture of a key or the dial: the hub's own while live, a preview otherwise. */
  function picture(c: Ctl): string | undefined {
    if (live) return hub.displays[displayKey(c.serial, c.ref.kind === 'dial' ? TARGET_DIAL : c.ref.kind === 'key' ? c.ref.index : -1)];
    return previewOf(info.get(c.id)?.cfg);
  }

  const owns = (cl: ClaimInfo, kind: ModuleKind) => {
    const o = cl.owns ?? {};
    switch (kind) {
      case 'keypad': return (o.keys?.length ?? 0) >= 15;
      case 'dial': return !!o.dial;
      case 'knob': return (o.knobs?.length ?? 0) >= 3;
      case 'roller': return (o.rollers?.length ?? 0) >= 2;
      case 'fader': return (o.faders?.length ?? 0) >= 5;
      default: return false;
    }
  };
  /** Modules a claim holds as a whole: dashed outline and tag. */
  const claimed = $derived(
    new Map(
      layout.placed.flatMap(({ m }) => {
        const cl = ctx.claims.find((x) => x.active && owns(x, m.kind));
        return cl ? [[m.serial, cl] as const] : [];
      }),
    ),
  );

  const markers = $derived.by(() => {
    const out: { id: string; kind: MarkerKind; x: number; y: number; title?: string }[] = [];
    for (const c of controls) {
      const o = info.get(c.id)?.owner;
      let kind: MarkerKind | undefined;
      let title: string | undefined;
      if (o?.kind === 'claim') {
        if (claimed.has(c.serial)) continue;
        [kind, title] = ['integration', `${o.claim.name} owns this control`];
      } else if (o?.kind === 'pinned') [kind, title] = ['pinned', 'Pinned everywhere (Global)'];
      else if (o?.kind === 'base') [kind, title] = ['inherited', `Inherited from ${o.profile} (base profile)`];
      if (kind) out.push({ id: c.id, kind, title, x: c.bounds[0] + c.bounds[2], y: c.bounds[1] });
    }
    return out;
  });

  const spots = $derived<Spot[]>(controls.map((c) => ({ id: c.id, sel: c.ref, cx: c.bounds[0] + c.bounds[2] / 2, cy: c.bounds[1] + c.bounds[3] / 2 })));
  const selected = (s: Selection) => sameSel(hub.selection, s);
  const tabStop = $derived(spots.find((s) => selected(s.sel))?.id ?? spots.find((s) => s.id.startsWith('key:'))?.id ?? spots[0]?.id);

  // ─── interaction ───
  let root: HTMLElement | undefined = $state();
  const pick = (s: Selection) => (e: Event) => {
    e.stopPropagation();
    hub.selection = s;
  };
  async function keydown(e: KeyboardEvent) {
    const dir = ({ ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' } as Record<string, Dir>)[e.key];
    if (!dir) return;
    const id = (e.target as HTMLElement).closest<HTMLElement>('[data-ctl]')?.dataset.ctl;
    if (!id) return;
    e.preventDefault();
    const next = neighbour(spots, id, dir);
    if (!next) return;
    hub.selection = next.sel;
    await tick();
    root?.querySelector<HTMLElement>(`[data-ctl="${next.id}"]`)?.focus();
  }

  // ─── fit the stage to the canvas ───
  let cw = $state(0);
  let ch = $state(0);
  const scale = $derived(cw && ch ? Math.min(cw / layout.w, ch / layout.h, MAX_SCALE) : 1);

  // ─── pulses: a profile was applied, a module was plugged in ───
  let applied = $state(0);
  let lastProfile: string | undefined;
  $effect(() => {
    const p = hub.state.profile;
    if (lastProfile !== undefined && p !== lastProfile) applied++;
    lastProfile = p;
  });
  let pulsing = $state<Record<string, boolean>>({});
  let seen: Set<string> | undefined;
  $effect(() => {
    const serials = hub.modules.map((m) => m.serial);
    if (!seen) {
      if (serials.length) seen = new Set(serials);
      return;
    }
    for (const s of serials) {
      if (seen.has(s)) continue;
      seen.add(s);
      pulsing[s] = true;
      setTimeout(() => delete pulsing[s], 2000);
    }
  });

  const knobCaption = (id: string) => {
    const cfg = info.get(id)?.cfg;
    return shortAction(cfg?.cw ?? cfg?.press ?? cfg?.ccw);
  };
  const rollerCaption = (id: string) => {
    const cfg = info.get(id)?.cfg;
    return shortAction(cfg?.away ?? cfg?.toward);
  };
  const cfgLabel = (id: string) => info.get(id)?.cfg?.label;
  const names: Record<string, string> = { key: 'Key', dial: 'Dial', knob: 'Knob', roller: 'Roller', fader: 'Fader' };
  const label = (c: Ctl) => (c.ref.kind === 'roller' ? `${c.ref.side === 'left' ? 'Left' : 'Right'} roller` : 'index' in c.ref ? `${names[c.ref.kind]} ${c.ref.index + 1}` : names[c.ref.kind]);
</script>

<div class="fit" bind:clientWidth={cw} bind:clientHeight={ch} role="presentation" onclick={() => (hub.selection = { kind: 'none' })}>
  <div class="frame" style="width:{layout.w * scale}px;height:{layout.h * scale}px">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="stage" bind:this={root} onkeydown={keydown} style="width:{layout.w}px;height:{layout.h}px;transform:scale({scale});--inv:{1 / scale}">
      <div class="world" style="transform:translate({layout.ox}px,{layout.oy}px)">
        {#each layout.placed as { m, matrix, unknown, bounds } (m.serial)}
          {@const [w, h] = FOOTPRINT[m.kind]}
          {@const mine = byModule.get(m.serial) ?? []}
          <div class="module {m.kind}" class:unknown style="width:{w * U}px;height:{h * U}px;transform:{cssMatrix(matrix)}">
            {#if m.kind === 'base'}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div class="base" class:live onclick={pick({ kind: 'module', serial: m.serial })}>
                {#key applied}{#if applied}<span class="applied marker-applied"></span>{/if}{/key}
              </div>
            {:else}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div class="chassis" onclick={pick({ kind: 'module', serial: m.serial })}></div>
            {/if}

            {#if m.kind === 'keypad'}
              <span class="brand">COOLER MASTER</span>
            {/if}

            {#each mine as c (c.id)}
              {@const [x, y, bw, bh] = c.box}
              {@const sel = selected(c.ref)}
              {@const src = picture(c)}
              {@const pos = `left:${x}px;top:${y}px;width:${bw}px;height:${bh}px`}
              {#if c.ref.kind === 'key'}
                <button type="button" class="ctl key" class:marker-selected={sel} data-ctl={c.id} aria-label={label(c)} aria-pressed={sel} tabindex={tabStop === c.id ? 0 : -1} style={pos} onclick={pick(c.ref)}>
                  {#if src}<img {src} alt="" draggable="false" style={live ? undefined : `transform:rotate(${-c.rot}deg)`} />{/if}
                  {#key hub.flashes[c.flash]}{#if hub.flashes[c.flash]}<span class="flash marker-pressed"></span>{/if}{/key}
                </button>
              {:else if c.ref.kind === 'dial'}
                <button type="button" class="ctl dial" class:marker-selected={sel} data-ctl={c.id} aria-label={label(c)} aria-pressed={sel} tabindex={tabStop === c.id ? 0 : -1} style={pos} onclick={pick(c.ref)}>
                  <span class="ticks"></span>
                  <span class="face">
                    {#if src}<img {src} alt="" draggable="false" style={live ? undefined : `transform:rotate(${-c.rot}deg)`} />{:else if cfgLabel(c.id)}<span class="dial-label">{cfgLabel(c.id)}</span>{/if}
                  </span>
                  {#key hub.flashes[c.flash]}{#if hub.flashes[c.flash]}<span class="flash marker-pressed"></span>{/if}{/key}
                </button>
              {:else if c.ref.kind === 'knob'}
                {@const led = ledColor('knob', c.ref.index)}
                <button type="button" class="ctl knob" class:marker-selected={sel} data-ctl={c.id} aria-label={label(c)} aria-pressed={sel} tabindex={tabStop === c.id ? 0 : -1} style="{pos};{led ? `--led:${led}` : ''}" onclick={pick(c.ref)}>
                  <span class="cap"></span>
                  {#key hub.flashes[c.flash]}{#if hub.flashes[c.flash]}<span class="flash marker-pressed"></span>{/if}{/key}
                </button>
                <span class="caption" style="left:{x - 16}px;top:{y + bh + 8}px;width:{bw + 32}px">{knobCaption(c.id)}</span>
              {:else if c.ref.kind === 'roller'}
                <button type="button" class="ctl roller" class:marker-selected={sel} data-ctl={c.id} aria-label={label(c)} aria-pressed={sel} tabindex={tabStop === c.id ? 0 : -1} style={pos} onclick={pick(c.ref)}>
                  {#key hub.flashes[c.flash]}{#if hub.flashes[c.flash]}<span class="flash marker-pressed"></span>{/if}{/key}
                </button>
                <span class="caption" style="left:{x}px;top:{y + bh + 8}px;width:{bw}px">{rollerCaption(c.id)}</span>
              {:else if c.ref.kind === 'fader'}
                {@const led = ledColor('fader', c.ref.index)}
                {@const pos01 = (hub.faders[`${c.serial}/${c.ref.index}`] ?? 50) / 100}
                <button type="button" class="ctl fader" class:marker-selected={sel} data-ctl={c.id} aria-label={label(c)} aria-pressed={sel} tabindex={tabStop === c.id ? 0 : -1} style="{pos};--pos:{pos01};{led ? `--led:${led}` : ''}" onclick={pick(c.ref)}>
                  <span class="track"></span>
                  <span class="thumb"></span>
                  {#key hub.flashes[c.flash]}{#if hub.flashes[c.flash]}<span class="flash marker-pressed"></span>{/if}{/key}
                </button>
              {/if}
            {/each}

            {#if hub.selection.kind === 'module' && hub.selection.serial === m.serial}
              <span class="module-ring marker-selected"></span>
            {/if}
            {#if pulsing[m.serial]}<span class="plugged"></span>{/if}
            {#if unknown}
              <span class="unknown-label">Position unknown · socket {m.socket ?? '?'}</span>
            {/if}
          </div>
          {#if claimed.has(m.serial)}
            {@const [bx, by, bw, bh] = bounds}
            <div class="claim" style="left:{bx}px;top:{by}px;width:{bw}px;height:{bh}px"><ClaimOutline label={claimed.get(m.serial)!.name} /></div>
          {/if}
        {/each}

        {#each markers as mk (mk.id)}
          <span class="mark" style="left:{mk.x}px;top:{mk.y}px"><CanvasMarker kind={mk.kind} title={mk.title} size={18} inline /></span>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .fit {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .frame {
    position: relative;
    flex: none;
  }
  .stage {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
  .world {
    position: absolute;
    left: 0;
    top: 0;
  }
  .module {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
  .base {
    position: absolute;
    inset: calc(var(--margin, 17px) * -1);
    border-radius: var(--hub-base-radius);
    background-color: var(--hub-base);
    background-image: radial-gradient(circle, var(--hub-socket) 3px, transparent 3.5px);
    background-size: 92px 92px;
    background-position: 17px 17px;
    box-shadow: var(--hub-base-shadow);
  }
  .base.live {
    box-shadow: 0 0 0 3px var(--color-accent), 0 0 28px 2px var(--color-accent-soft), var(--hub-base-shadow);
  }
  .applied {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    pointer-events: none;
  }
  .chassis {
    position: absolute;
    inset: 3px;
    border-radius: var(--hub-chassis-radius);
    background: var(--hub-chassis);
    box-shadow: var(--hub-chassis-shadow);
  }
  .unknown .chassis {
    outline: 1px dashed var(--color-warning);
  }
  .unknown-label {
    position: absolute;
    left: 4px;
    top: -18px;
    white-space: nowrap;
    font: 500 11px / 1 var(--type-body-family);
    color: var(--color-warning);
  }
  .brand {
    position: absolute;
    left: 0;
    right: 0;
    top: 244px;
    text-align: center;
    font: 700 7px / 1 var(--type-body-family);
    letter-spacing: 0.32em;
    color: var(--hub-brand);
    pointer-events: none;
  }
  .module-ring {
    position: absolute;
    inset: 0;
    border-radius: calc(var(--hub-chassis-radius) + 3px);
    pointer-events: none;
  }
  .plugged {
    position: absolute;
    inset: 0;
    border-radius: calc(var(--hub-chassis-radius) + 3px);
    pointer-events: none;
    animation: applied-pulse var(--motion-applied-pulse) 3;
  }
  .claim {
    position: absolute;
    pointer-events: none;
  }
  .mark {
    position: absolute;
    z-index: 5;
    display: grid;
    transform: translate(-50%, -50%) scale(var(--inv));
  }

  /* controls */
  .ctl {
    position: absolute;
    padding: 0;
    border: 0;
    color: var(--hub-key-text);
    cursor: pointer;
  }
  .ctl:hover:not(.marker-selected) {
    outline: 1px solid var(--color-border-2);
    outline-offset: 3px;
  }
  .flash {
    position: absolute;
    inset: -3px;
    border-radius: inherit;
    pointer-events: none;
    animation: fade-out 450ms ease-out forwards;
  }
  @keyframes fade-out {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }
  .key {
    border-radius: var(--hub-key-radius);
    background: var(--hub-empty);
    box-shadow: var(--hub-key-frame);
  }
  .key img {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: inherit;
    object-fit: cover;
    pointer-events: none;
  }
  .dial {
    border-radius: 50%;
    background: transparent;
  }
  .dial .face {
    position: absolute;
    inset: 16px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 50%;
    background: var(--hub-dial-face);
    box-shadow: var(--hub-dial-bezel);
  }
  .dial .face img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
  }
  .dial-label {
    font-weight: 700;
    color: var(--hub-key-text);
  }
  .dial .ticks {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    pointer-events: none;
    color: var(--hub-dial-ring-1);
    background: repeating-conic-gradient(currentColor 0 1.3deg, transparent 1.3deg 6deg);
    mask: radial-gradient(farthest-side, transparent 80%, currentColor 81%);
  }
  .knob {
    border-radius: 50%;
    background: transparent;
    box-shadow: 0 0 0 3px var(--led, var(--hub-chassis-stroke));
  }
  .knob .cap {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: linear-gradient(160deg, var(--hub-knob-light), var(--hub-knob-dark));
  }
  .knob .cap::after {
    content: '';
    position: absolute;
    left: calc(50% - 1.5px);
    top: 6px;
    width: 3px;
    height: 12px;
    border-radius: 2px;
    background: var(--hub-knob-mark);
  }
  .caption {
    position: absolute;
    overflow: hidden;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 400 10px / 1.2 var(--type-body-family);
    color: var(--hub-caption);
    pointer-events: none;
  }
  .roller {
    border-radius: var(--hub-roller-radius);
    background:
      repeating-linear-gradient(to bottom, transparent 0 3px, var(--hub-roller-dark) 3px 5px),
      linear-gradient(to bottom, var(--hub-roller-light), var(--hub-roller-dark));
    box-shadow: var(--hub-roller-shadow);
  }
  .fader {
    border-radius: 12px;
    background: transparent;
  }
  .fader .track {
    position: absolute;
    left: calc(50% - 1.5px);
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 2px;
    background: var(--led, var(--hub-roller-light));
    opacity: 0.8;
  }
  .fader .thumb {
    position: absolute;
    left: 5px;
    right: 5px;
    height: 20px;
    top: calc(10px + (1 - var(--pos)) * (100% - 40px));
    border-radius: 5px;
    background: linear-gradient(to bottom, var(--hub-knob-light), var(--hub-knob-dark));
    box-shadow: 0 0 0 1px var(--hub-chassis-stroke);
  }
</style>
