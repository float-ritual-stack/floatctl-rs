/**
 * Project string canonicalizer — used at capture write, active_context
 * query, and backfill.
 *
 * Fixes drift observed at SQL layer in active_context_stream (Supabase MCP
 * audit, 2026-04-20): trailing " |" from parser boundary bugs, case
 * variants, whitespace around slashes. Keeps true sub-project hierarchy
 * intact (`floatty-ai-explorer` is NOT collapsed to `floatty`).
 *
 * Composes with AnnotationParser.normalizeProjectName — canonicalize runs
 * BEFORE canonical/alias lookup so drift variants resolve correctly.
 */
export function canonicalizeProject(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const trimmed = raw
    .trim()
    .replace(/\s*\|\s*$/, '')       // strip trailing " |" with surrounding whitespace
    .replace(/\s+\/\s+/g, '/')      // "float / consciousness-tech" → "float/consciousness-tech"
    .toLowerCase();
  return trimmed.length > 0 ? trimmed : null;
}
