/** Ignore a second acceptance before React has painted the first insertion. */
export function acceptCompletionOnce(
  acceptedAt: { current: string | null },
  position: string,
  accept: () => void
) {
  if (acceptedAt.current === position) return;
  acceptedAt.current = position;
  accept();
}
