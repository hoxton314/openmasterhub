// Fuzzy matching for the command palette: every query character must appear in order; runs of
// consecutive characters and matches at word starts score higher.

export interface Scored<T> {
  item: T;
  score: number;
}

/** Score `text` against `query` (higher is better), or `undefined` when it does not match. */
export function fuzzy(query: string, text: string): number | undefined {
  const q = query.toLowerCase().replace(/\s+/g, '');
  if (!q) return 0;
  const t = text.toLowerCase();
  let score = 0;
  let at = -1;
  let run = 0;
  for (const ch of q) {
    const i = t.indexOf(ch, at + 1);
    if (i < 0) return undefined;
    run = i === at + 1 ? run + 1 : 0;
    const wordStart = i === 0 || /[\s\-_/·:.]/.test(t[i - 1]);
    score += 1 + run * 2 + (wordStart ? 3 : 0) - Math.min(i - at - 1, 6) * 0.1;
    at = i;
  }
  if (t.startsWith(q)) score += 6;
  else if (t.includes(q)) score += 3;
  return score - t.length * 0.01;
}

/** Items matching `query`, best first; with an empty query the input order is kept. */
export function rank<T>(query: string, items: T[], text: (item: T) => string): T[] {
  if (!query.trim()) return items;
  return items
    .map((item) => ({ item, score: fuzzy(query, text(item)) }))
    .filter((s): s is Scored<T> => s.score !== undefined)
    .sort((a, b) => b.score - a.score)
    .map((s) => s.item);
}
