import { describe, expect, it } from 'vitest';
import { fuzzy, rank } from './search';

describe('fuzzy', () => {
  it('needs every character in order', () => {
    expect(fuzzy('blnd', 'Blender')).toBeDefined();
    expect(fuzzy('xyz', 'Blender')).toBeUndefined();
    expect(fuzzy('reb', 'Blender')).toBeUndefined();
  });
  it('prefers prefixes and word starts', () => {
    expect(fuzzy('ble', 'Blender')!).toBeGreaterThan(fuzzy('ble', 'Obsidian bleak')!);
    expect(fuzzy('sw', 'Switch setup')!).toBeGreaterThan(fuzzy('sw', 'Passwords')!);
  });
  it('ranks best first and keeps order for an empty query', () => {
    const items = ['Restart service', 'Render', 'Desktop'];
    expect(rank('', items, (s) => s)).toEqual(items);
    expect(rank('ren', items, (s) => s)[0]).toBe('Render');
    expect(rank('qq', items, (s) => s)).toEqual([]);
  });
});
