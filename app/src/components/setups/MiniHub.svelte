<!-- A small dark MasterHUB (always dark hardware, in every theme) whose dial and rollers can be picked as switch triggers. -->
<script lang="ts">
  export type HubControl = 'dial' | 'left' | 'right';

  let {
    selected,
    active = [],
    onpick,
  }: { selected?: HubControl; /** Controls that already switch. */ active?: HubControl[]; onpick: (c: HubControl) => void } = $props();

  const LABEL: Record<HubControl, string> = { dial: 'Dial', left: 'Left roller', right: 'Right roller' };
</script>

<div class="hub" role="group" aria-label="Mini hub: pick a control to switch with">
  <div class="keys" aria-hidden="true">
    {#each Array(15) as _, i (i)}<span></span>{/each}
  </div>
  <button type="button" class="dial" class:selected={selected === 'dial'} class:active={active.includes('dial')} aria-pressed={selected === 'dial'} aria-label="{LABEL.dial}{active.includes('dial') ? ' (switches)' : ''}" onclick={() => onpick('dial')}></button>
  <div class="knobs" aria-hidden="true"><span></span><span></span><span></span></div>
  {#each ['left', 'right'] as side (side)}
    <button type="button" class="roller {side}" class:selected={selected === side} class:active={active.includes(side as HubControl)} aria-pressed={selected === side} aria-label="{LABEL[side as HubControl]}{active.includes(side as HubControl) ? ' (switches)' : ''}" onclick={() => onpick(side as HubControl)}></button>
  {/each}
</div>

<style>
  .hub {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 70px;
    grid-template-rows: auto auto auto;
    gap: 8px 10px;
    width: 224px;
    padding: 10px;
    background: var(--hub-base);
    border-radius: var(--hub-base-radius);
    box-shadow: var(--hub-base-shadow);
  }
  .keys {
    grid-row: 1 / 3;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 3px;
    padding: 6px;
    background: var(--hub-chassis);
    border-radius: var(--hub-chassis-radius);
    box-shadow: var(--hub-chassis-shadow);
  }
  .keys span {
    aspect-ratio: 1;
    background: var(--hub-key);
    border-radius: var(--hub-key-radius);
  }
  button {
    padding: 0;
    border: 0;
    cursor: pointer;
    outline-offset: 3px;
  }
  .dial {
    width: 54px;
    height: 54px;
    justify-self: center;
    background: var(--hub-dial-face);
    border-radius: 50%;
    box-shadow: 0 0 0 3px var(--hub-dial-ring-2), 0 0 0 4px var(--hub-dial-ring-1);
  }
  .knobs {
    display: flex;
    justify-content: space-between;
    padding: 0 4px;
  }
  .knobs span {
    width: 14px;
    height: 14px;
    background: radial-gradient(circle at 35% 30%, var(--hub-knob-light), var(--hub-knob-dark));
    border-radius: 50%;
  }
  .roller {
    grid-row: 3;
    height: 16px;
    background: linear-gradient(var(--hub-roller-light), var(--hub-roller-dark));
    border-radius: var(--hub-roller-radius);
    box-shadow: var(--hub-roller-shadow);
  }
  .left {
    grid-column: 1;
  }
  .right {
    grid-column: 1 / 3;
    grid-row: 4;
  }
  .active {
    box-shadow: 0 0 0 2px var(--color-accent);
  }
  .dial.active {
    box-shadow: 0 0 0 3px var(--hub-dial-ring-2), 0 0 0 5px var(--color-accent);
  }
  .selected {
    outline: 2px solid var(--color-focus);
  }
</style>
