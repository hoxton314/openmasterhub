// Shared UI state: the modal dialog.

export interface DialogSpec {
  title: string;
  body?: string;
  /** Show a text field with this initial value. */
  value?: string;
  placeholder?: string;
  suggestions?: string[];
  ok: string;
  danger?: boolean;
  input: boolean;
  done: (value: string) => void;
}

export const ui = $state({
  dialog: null as DialogSpec | null,

  ask(spec: Omit<DialogSpec, 'input'>) {
    ui.dialog = { ...spec, value: spec.value ?? '', input: true };
  },

  confirm(spec: Omit<DialogSpec, 'input' | 'done'> & { done: () => void }) {
    ui.dialog = { ...spec, danger: true, input: false, done: () => spec.done() };
  },
});
