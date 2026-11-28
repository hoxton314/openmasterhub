<!--
  A mini hub for template cards and the wizard: the first page's keys, the dial, knobs, faders and
  rollers a profile sets. Controls the profile leaves empty but `base` sets get the inherited marker.
  Hardware colours come from the --hub-* tokens; key faces keep their own (data) colours.
-->
<script lang="ts">
  import type { KeyConfig, Profile } from '../../lib/types';
  import CanvasMarker from '../ui/CanvasMarker.svelte';

  let {
    profile,
    base,
    keys,
    unit = 22,
  }: {
    profile?: Profile;
    base?: Profile;
    /** Show just these keys (a page), without the other modules. */
    keys?: KeyConfig[];
    /** Key edge in px. */
    unit?: number;
  } = $props();

  const pageKeys = $derived(keys ?? profile?.page?.[0]?.key ?? []);
  const cells = $derived(Array.from({ length: 15 }, (_, i) => pageKeys.find((k) => k.index === i)));
  const only = $derived(!!keys);

  const has = (p: Profile | undefined) => ({
    dial: !!p?.dial,
    knob: !!p?.knob?.length,
    fader: !!p?.fader?.length,
    roller: !!p?.roller?.length,
  });
  const own = $derived(has(profile));
  const inherited = $derived.by(() => {
    const b = has(base);
    return { dial: !own.dial && b.dial, knob: !own.knob && b.knob, fader: !own.fader && b.fader, roller: !own.roller && b.roller };
  });
  const show = $derived({
    dial: own.dial || inherited.dial,
    knob: own.knob || inherited.knob,
    fader: own.fader,
    roller: own.roller || inherited.roller,
  });
  const label = (k: KeyConfig | undefined) => k?.label ?? (k?.widget ? k.widget.kind.replace(/_/g, ' ') : '');
</script>

<div class="hub" style="--u:{unit}px" role="img" aria-label="{profile?.name ?? 'Page'} preview">
  <div class="keypad">
    {#each cells as k, i (i)}
      <span class="key" class:empty={!k} style={k?.background ? `background:${k.background}` : ''}>
        {#if k}<span class="t" style={k.text_color ? `color:${k.text_color}` : ''}>{label(k)}</span>{/if}
      </span>
    {/each}
  </div>
  {#if !only && (show.dial || show.knob || show.fader || show.roller)}
    <div class="side">
      {#if show.dial}
        <span class="dial">
          <span class="t">{profile?.dial?.label ?? ''}</span>
          {#if inherited.dial}<CanvasMarker kind="inherited" size={14} />{/if}
        </span>
      {/if}
      {#if show.knob}
        <span class="knobs">
          {#each [0, 1, 2] as i}<span class="knob"></span>{/each}
          {#if inherited.knob}<CanvasMarker kind="inherited" size={14} />{/if}
        </span>
      {/if}
      {#if show.fader}
        <span class="faders">{#each profile?.fader ?? [] as f (f.index)}<span class="fader"></span>{/each}</span>
      {/if}
      {#if show.roller}
        <span class="roller">{#if inherited.roller}<CanvasMarker kind="inherited" size={14} />{/if}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .hub {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--u) * 0.35);
    padding: calc(var(--u) * 0.3);
    background: var(--hub-chassis);
    border-radius: var(--hub-chassis-radius);
    box-shadow: var(--hub-chassis-shadow);
  }
  .keypad {
    display: grid;
    grid-template-columns: repeat(5, var(--u));
    grid-auto-rows: var(--u);
    gap: calc(var(--u) * 0.1);
  }
  .key {
    display: grid;
    place-items: center;
    overflow: hidden;
    background: var(--hub-key);
    border-radius: calc(var(--hub-key-radius) * var(--u) / 28);
    box-shadow: var(--hub-key-frame);
  }
  .key.empty {
    background: var(--hub-empty);
  }
  .t {
    max-width: 100%;
    padding: 0 1px;
    overflow: hidden;
    text-overflow: clip;
    white-space: nowrap;
    font-size: max(calc(var(--u) * 0.24), 5px);
    line-height: 1;
    color: var(--hub-key-text);
    text-transform: lowercase;
  }
  .side {
    display: grid;
    justify-items: center;
    gap: calc(var(--u) * 0.3);
  }
  .dial {
    position: relative;
    display: grid;
    place-items: center;
    width: calc(var(--u) * 1.7);
    height: calc(var(--u) * 1.7);
    background: var(--hub-dial-face);
    border-radius: 50%;
    box-shadow: 0 0 0 calc(var(--u) * 0.08) var(--hub-dial-ring-1);
  }
  .dial .t {
    color: var(--hub-dial-caption);
    text-align: center;
  }
  .knobs {
    position: relative;
    display: flex;
    gap: calc(var(--u) * 0.2);
  }
  .knob {
    width: calc(var(--u) * 0.5);
    height: calc(var(--u) * 0.5);
    background: var(--hub-knob-light);
    border-radius: 50%;
  }
  .faders {
    display: flex;
    gap: calc(var(--u) * 0.15);
  }
  .fader {
    width: calc(var(--u) * 0.16);
    height: calc(var(--u) * 1.1);
    background: var(--hub-roller-light);
    border-radius: 99px;
  }
  .roller {
    position: relative;
    width: calc(var(--u) * 1.4);
    height: calc(var(--u) * 0.4);
    background: var(--hub-roller-light);
    border-radius: var(--hub-roller-radius);
  }
</style>
