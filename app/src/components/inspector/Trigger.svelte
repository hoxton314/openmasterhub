<!-- One trigger row (Press, Turn left…) that opens its action editor in place. Only one row of an
     inspector is open at a time (`open` is the id of the open one). -->
<script lang="ts">
  import { describe, edit } from '../../lib/hub.svelte';
  import type { Action, Config } from '../../lib/types';
  import { prettyKeys } from '../../lib/keys';
  import type { Icon } from '../ui/icon';
  import TriggerRow from '../ui/TriggerRow.svelte';
  import ActionEditor from './ActionEditor.svelte';

  let {
    id,
    label,
    icon,
    value,
    open = $bindable(null),
    set,
    fader = false,
    liveValue,
    defaultVolume,
    readonly = false,
  }: {
    id: string;
    label: string;
    icon: Icon;
    value: Action | undefined;
    open?: string | null;
    /** Writes the action into the config (inside `edit`). */
    set: (c: Config, a: Action | undefined) => void;
    fader?: boolean;
    liveValue?: number;
    defaultVolume?: string;
    readonly?: boolean;
  } = $props();

  const keys = $derived(value && 'keys' in value ? (typeof value.keys === 'string' ? { combo: value.keys, hold: undefined } : value.keys) : undefined);
  const summary = $derived(keys?.hold ? `${prettyKeys(keys.combo).join(' + ')} · hold ${keys.hold} ms` : value ? describe(value) : undefined);
</script>

<TriggerRow
  {label}
  {icon}
  combo={keys && !keys.hold ? keys.combo : undefined}
  summary={keys?.hold || !keys ? summary : undefined}
  active={open === id}
  onclick={() => (open = open === id ? null : id)}
>
  <fieldset class="lock" disabled={readonly}>
    <ActionEditor {value} {fader} {liveValue} {defaultVolume} onchange={(a) => edit((c) => set(c, a), { label: `change ${label.toLowerCase()}`, coalesce: `action:${id}` })} />
  </fieldset>
</TriggerRow>

<style>
  .lock {
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }
</style>
