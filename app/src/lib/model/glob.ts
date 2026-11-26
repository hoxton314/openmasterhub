// Port of `glob_match` in crates/masterhub-core/src/config.rs: case-insensitive, `*` wildcards.
// Keep the two in step; glob.test.ts mirrors the Rust tests.

export function globMatch(pattern: string, text: string): boolean {
  const p = pattern.toLowerCase();
  const t = text.toLowerCase();
  const parts = p.split('*');
  if (parts.length === 1) return p === t;
  let rest = t;
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    if (i === 0) {
      if (!rest.startsWith(part)) return false;
      rest = rest.slice(part.length);
    } else if (i === parts.length - 1) {
      return rest.endsWith(part);
    } else {
      const pos = rest.indexOf(part);
      if (pos < 0) return false;
      rest = rest.slice(pos + part.length);
    }
  }
  return true;
}

/** The first of `patterns` matching `text`, with its index. */
export function firstMatch(patterns: readonly string[] | undefined, text: string): { pattern: string; index: number } | undefined {
  const index = (patterns ?? []).findIndex((p) => globMatch(p, text));
  return index < 0 ? undefined : { pattern: patterns![index], index };
}
