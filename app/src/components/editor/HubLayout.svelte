<!--
  The connected hub as a small read-only drawing: the same layout the editor canvas shows (Base
  rotation, module sockets and rotations from `layoutModules`), without pictures or controls.
  Modules the service has not placed are left out. `highlight` rings one module by serial.
-->
<script lang="ts">
  import { DIAL_PX, FOOTPRINT, KNOB_PX, ROLLER_PX, U, cssMatrix, faderBox, keyBox, layoutModules, type Box } from '../../lib/geometry';
  import { hub } from '../../lib/hub.svelte';
  import type { ModuleState } from '../../lib/types';

  let { modules = hub.modules, rotation = hub.state.rotation ?? 90, width = 260, height = 260, highlight }: { modules?: ModuleState[]; rotation?: number; width?: number; height?: number; highlight?: string } = $props();

  const layout = $derived(layoutModules(modules, rotation, 14, false));
  const scale = $derived(Math.min(width / layout.w, height / layout.h));
  const kit = (m: ModuleState): Box[] => {
    switch (m.kind) {
      case 'keypad': return Array.from({ length: 15 }, (_, i) => keyBox(i));
      case 'knob': return KNOB_PX.centers.map(([cx, cy]) => [cx - KNOB_PX.r, cy - KNOB_PX.r, KNOB_PX.r * 2, KNOB_PX.r * 2] as Box);
      case 'roller': return [ROLLER_PX.left, ROLLER_PX.right];
      case 'fader': return Array.from({ length: 5 }, (_, i) => faderBox(i));
      default: return [];
    }
  };
</script>

<div class="frame" style="width:{layout.w * scale}px;height:{layout.h * scale}px" aria-hidden="true">
  <div class="stage" style="width:{layout.w}px;height:{layout.h}px;transform:scale({scale})">
    <div class="world" style="transform:translate({layout.ox}px,{layout.oy}px)">
      {#each layout.placed as { m, matrix } (m.serial)}
        {@const [w, h] = FOOTPRINT[m.kind]}
        <div class="module" style="width:{w * U}px;height:{h * U}px;transform:{cssMatrix(matrix)}">
          {#if m.kind === 'base'}
            <div class="base"></div>
          {:else}
            <div class="chassis"></div>
            {#each kit(m) as [x, y, bw, bh]}
              <span class="ctl {m.kind}" style="left:{x}px;top:{y}px;width:{bw}px;height:{bh}px"></span>
            {/each}
            {#if m.kind === 'dial'}
              <span class="ctl dial" style="left:{DIAL_PX.cx - DIAL_PX.ring}px;top:{DIAL_PX.cy - DIAL_PX.ring}px;width:{DIAL_PX.ring * 2}px;height:{DIAL_PX.ring * 2}px"><span class="face"></span></span>
            {/if}
            {#if highlight === m.serial}<span class="ring"></span>{/if}
          {/if}
          {#if m.kind === 'base' && highlight === m.serial}<span class="ring base-ring"></span>{/if}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .frame {
    position: relative;
    flex: none;
    border-radius: var(--radius-lg);
  }
  .stage {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
  .world,
  .module {
    position: absolute;
    left: 0;
    top: 0;
  }
  .module {
    transform-origin: 0 0;
  }
  .base {
    position: absolute;
    inset: -17px;
    border-radius: var(--hub-base-radius);
    background-color: var(--hub-base);
    background-image: radial-gradient(circle, var(--hub-socket) 3px, transparent 3.5px);
    background-size: 92px 92px;
    background-position: 17px 17px;
    box-shadow: var(--hub-base-shadow);
  }
  .chassis {
    position: absolute;
    inset: 3px;
    border-radius: var(--hub-chassis-radius);
    background: var(--hub-chassis);
    box-shadow: var(--hub-chassis-shadow);
  }
  .ring {
    position: absolute;
    inset: -2px;
    border-radius: calc(var(--hub-chassis-radius) + 3px);
    box-shadow: 0 0 0 5px var(--color-accent), 0 0 24px 4px var(--color-accent-soft);
    pointer-events: none;
  }
  .base-ring {
    inset: -19px;
    border-radius: var(--hub-base-radius);
  }
  .ctl {
    position: absolute;
  }
  .keypad {
    border-radius: var(--hub-key-radius);
    background: var(--hub-empty);
    box-shadow: var(--hub-key-frame);
  }
  .dial {
    border-radius: 50%;
    box-shadow: 0 0 0 3px var(--hub-dial-ring-2), 0 0 0 4px var(--hub-dial-ring-1);
  }
  .dial .face {
    position: absolute;
    inset: 16px;
    border-radius: 50%;
    background: var(--hub-dial-face);
    box-shadow: var(--hub-dial-bezel);
  }
  .knob {
    border-radius: 50%;
    background: linear-gradient(160deg, var(--hub-knob-light), var(--hub-knob-dark));
    box-shadow: 0 0 0 3px var(--hub-chassis-stroke);
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
    background: var(--hub-roller-dark);
  }
</style>
