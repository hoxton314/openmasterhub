<!-- An area in the 64 px rail: icon 20 + 10 px label; active = selection tint + accent (Signal: fill). -->
<script lang="ts">
  import type { Icon } from './icon';

  let { icon: Ic, label, active = false, onclick, badge }: { icon: Icon; label: string; active?: boolean; onclick: () => void; badge?: boolean } = $props();
</script>

<button type="button" class:active aria-current={active ? 'page' : undefined} aria-label={label} {onclick}>
  <span class="ic"><Ic size={20} />{#if badge}<span class="badge"></span>{/if}</span>
  <span class="label">{label}</span>
</button>

<style>
  button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    width: 52px;
    padding: 7px 0 5px;
    color: var(--idle-item-fg);
    background: transparent;
    border: 0;
    border-radius: var(--radius-rail);
    cursor: pointer;
    transition: background var(--motion-fast), color var(--motion-fast);
  }
  button:hover:not(.active) {
    background: var(--color-selection);
    color: var(--color-text-1);
  }
  .active {
    background: var(--active-item-bg);
    color: var(--active-item-fg);
  }
  .ic {
    position: relative;
    display: inline-flex;
  }
  .badge {
    position: absolute;
    top: -2px;
    right: -3px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-warning);
  }
  .label {
    font: var(--button-weight) var(--type-micro-size) / 1.2 var(--type-micro-family);
  }
</style>
