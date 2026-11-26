<!--
  Key capture for Keys actions. Idle: caps + Record; listening: an accent box with the modifiers
  held so far and "Press keys · Esc cancels". A text fallback takes any combo the recorder can't
  (media keys, keys the window manager grabs). With `onhold`, a Tap/Hold switch and hold time.
-->
<script lang="ts">
  import { Circle } from 'lucide-svelte';
  import { comboOf, modifiers, prettyKeys } from '../../lib/keys';
  import Button from './Button.svelte';
  import Kbd from './Kbd.svelte';
  import KeyCombo from './KeyCombo.svelte';
  import NumberField from './NumberField.svelte';
  import Segmented from './Segmented.svelte';

  let {
    value,
    onchange,
    hold,
    onhold,
    label = 'Shortcut',
    fallback = true,
    requireModifier = false,
  }: {
    value: string | undefined;
    onchange: (combo: string) => void;
    /** Hold time in ms; undefined = tap. */
    hold?: number;
    /** Shows Tap/Hold when given (hide it without the `hold` capability). */
    onhold?: (ms: number | undefined) => void;
    label?: string;
    /** Show the "Or type it" text field. */
    fallback?: boolean;
    /** Global shortcuts need a modifier. */
    requireModifier?: boolean;
  } = $props();

  let listening = $state(false);
  let held = $state<string[]>([]);
  let typed = $state('');
  let box: HTMLElement | undefined = $state();
  $effect(() => {
    typed = value ?? '';
  });

  function start() {
    listening = true;
    held = [];
    queueMicrotask(() => box?.focus());
  }

  function key(e: KeyboardEvent) {
    if (!listening) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey) {
      listening = false;
      return;
    }
    const combo = comboOf(e);
    if (!combo) {
      held = modifiers(e);
      return;
    }
    if (requireModifier && modifiers(e).length === 0) return;
    listening = false;
    onchange(combo);
  }
  function keyup(e: KeyboardEvent) {
    if (listening) held = modifiers(e);
  }
</script>

<div class="capture">
  <div class="label">{label}</div>
  {#if listening}
    <div class="box listening" tabindex="-1" role="textbox" aria-label="Press the keys" bind:this={box} onkeydown={key} onkeyup={keyup} onblur={() => (listening = false)}>
      {#each held as m, i}
        {#if i > 0}<span class="plus">+</span>{/if}<Kbd>{prettyKeys(m)[0]}</Kbd>
      {/each}
      {#if held.length}<span class="plus">+</span>{/if}
      <span class="caret"></span>
      <span class="prompt">Press keys · Esc cancels</span>
    </div>
  {:else}
    <div class="box">
      <KeyCombo combo={value} />
      <Button variant="ghost" size="sm" icon={Circle} onclick={start}>Record</Button>
    </div>
  {/if}
  {#if fallback}
    <div class="typed">
      Or type it:
      <input class="mono" bind:value={typed} spellcheck="false" placeholder="ctrl+shift+t" aria-label="{label} as text" onchange={() => typed.trim() && onchange(typed.trim().toLowerCase())} />
      <span>media keys like <code>playpause</code></span>
    </div>
  {/if}
  {#if onhold}
    <div class="hold">
      <Segmented
        label="Tap or hold"
        options={[{ value: 'tap', label: 'Tap' }, { value: 'hold', label: 'Hold' }]}
        value={hold ? 'hold' : 'tap'}
        onchange={(v) => onhold(v === 'hold' ? (hold ?? 250) : undefined)}
      />
      {#if hold}
        <div class="ms"><NumberField value={hold} min={20} max={10000} step={10} unit="ms" onchange={(v) => onhold(v ?? 250)} /></div>
        <span class="note">Held for {hold} ms, then released</span>
      {:else}
        <span class="note">Sent once, released at once</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .capture {
    display: grid;
    gap: 6px;
  }
  .label {
    font-size: var(--type-small-size);
    font-weight: 500;
    color: var(--color-text-2);
  }
  .box {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    min-height: calc(var(--size-kbd) + 14px);
    padding: 6px 6px 6px 10px;
    background: var(--button-secondary-bg);
    border: var(--field-border);
    border-radius: var(--radius-control);
  }
  .listening {
    justify-content: flex-start;
    padding: 6px 10px;
    background: var(--color-selection);
    border: 2px solid var(--color-accent);
    outline: none;
  }
  .caret {
    width: 2px;
    height: 18px;
    background: var(--color-accent);
    animation: blink 1s steps(1) infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  .prompt {
    margin-left: 4px;
    font-size: var(--type-chip-size);
    color: var(--color-text-2);
  }
  .plus {
    color: var(--color-text-3);
  }
  .typed {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 6px;
    font-size: var(--type-chip-size);
    color: var(--color-text-3);
  }
  .typed input {
    width: 120px;
    height: 22px;
    padding: 0 6px;
    font: var(--type-mono-weight) var(--type-chip-size) var(--type-mono-family);
    color: var(--color-text-1);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xs);
    outline: none;
    user-select: text;
  }
  .typed input:focus {
    border-color: var(--color-accent);
  }
  .typed code {
    font-size: var(--type-chip-size);
  }
  .hold {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .ms {
    width: 110px;
  }
  .note {
    font-size: var(--type-small-size);
    color: var(--color-text-3);
  }
</style>
