import { beforeEach, describe, expect, it } from 'vitest';
import { setUserThemes, theme } from './theme.svelte';

const file = (id: string, extra: object, tokens: object = {}) => ({ file: `${id}.json`, json: JSON.stringify({ schemaVersion: 1, id, name: id, appearance: 'dark', inherits: 'dark', tokens, ...extra }) });
const listed = () => theme.list.filter((t) => t.source === 'user').map((t) => t.id).sort();

beforeEach(() => setUserThemes([]));

describe('setUserThemes', () => {
  it('keeps valid themes and reports broken ones', () => {
    setUserThemes([file('good', {}, { color: { accent: '#FF00AA' } }), file('bad', {}, { color: { bg: '{color.nope}' } })]);
    expect(listed()).toEqual(['good']);
    expect(theme.problems.map((p) => p.file)).toEqual(['bad.json']);
  });

  it('drops children of a broken parent, whatever the file order', () => {
    // `child` overrides the token whose reference is dangling in `parent`, so it resolves while
    // `parent` is still listed and only breaks once `parent` has been dropped.
    const broken = { color: { bg: '{color.nope}' } };
    const parent = file('parent', {}, broken);
    const child = file('child', { inherits: 'parent' }, { color: { bg: '#101010' } });
    const grandchild = file('grandchild', { inherits: 'child' });
    for (const order of [[parent, child, grandchild], [grandchild, child, parent], [child, grandchild, parent]]) {
      setUserThemes(order);
      expect(listed()).toEqual([]);
      expect(theme.problems.map((p) => p.file).sort()).toEqual(['child.json', 'grandchild.json', 'parent.json']);
    }
  });
});
