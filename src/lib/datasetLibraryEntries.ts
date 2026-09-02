/**
 * Shared by the tutorial and recent-document rows in the Data library: a
 * path that exists but that FrameWork cannot open for reading (a macOS TCC
 * deny, permissions changed underneath it, ...) stays listed rather than
 * vanishing, but reads as disabled and says so instead of silently doing
 * nothing when clicked.
 */
export interface LibraryEntryAvailability {
  exists: boolean;
  readable: boolean;
}

export interface LibraryEntryState {
  /** True once the row should refuse clicks and render disabled. */
  disabled: boolean;
  /** Inline text for after the entry's name, or null when none is needed. */
  suffix: string | null;
}

const UNREADABLE_SUFFIX = "can't be read";

/**
 * `readable` defaults to true for callers that have not populated the field
 * yet — an older cached recent-documents entry, or a test fixture — so an
 * entry already known to exist does not read as broken before the field is
 * backfilled.
 */
export function libraryEntryState({
  exists,
  readable = true,
}: Partial<LibraryEntryAvailability> & Pick<LibraryEntryAvailability, "exists">): LibraryEntryState {
  const disabled = exists && !readable;
  return { disabled, suffix: disabled ? UNREADABLE_SUFFIX : null };
}
