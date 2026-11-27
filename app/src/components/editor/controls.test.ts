import { describe, expect, it } from 'vitest';
import { describeControls, neighbour, type Spot } from './controls';

const spot = (id: string, cx: number, cy: number): Spot => ({ id, sel: { kind: 'none' }, cx, cy });

describe('neighbour', () => {
  const spots = [spot('a', 0, 0), spot('b', 70, 0), spot('c', 140, 0), spot('d', 0, 70), spot('e', 300, 300)];
  it('moves to the nearest control in the direction', () => {
    expect(neighbour(spots, 'a', 'right')?.id).toBe('b');
    expect(neighbour(spots, 'b', 'left')?.id).toBe('a');
    expect(neighbour(spots, 'a', 'down')?.id).toBe('d');
  });
  it('stays put at an edge', () => {
    expect(neighbour(spots, 'a', 'left')).toBeUndefined();
    expect(neighbour(spots, 'a', 'up')).toBeUndefined();
  });
});

describe('describeControls', () => {
  it('groups controls for the Why chain', () => {
    expect(describeControls([{ kind: 'key', index: 8 }])).toBe('Key 9');
    expect(describeControls([0, 1, 2].map((index) => ({ kind: 'knob' as const, index })).concat() as never)).toBe('Knobs');
    expect(describeControls([{ kind: 'knob', index: 0 }, { kind: 'knob', index: 1 }, { kind: 'knob', index: 2 }, { kind: 'roller', side: 'right' }])).toBe('Knobs and right roller');
  });
});
