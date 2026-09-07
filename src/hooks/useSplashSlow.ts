import { useEffect, useState } from "react";

/**
 * Whether the splash has been up long enough to say why it might be.
 *
 * A slow first launch is usually macOS waiting on a permission dialog
 * (Documents-folder access, triggered by the first tutorial-library read)
 * that can be hidden behind the splash: the app sat on "Opening your
 * canvas…" for a minute while the dialog waited for a click nobody knew to
 * make. After four seconds the wait is more likely that than a slow disk,
 * so the splash gets a second line saying so rather than staying silent.
 */
export function useSplashSlow(waiting: boolean): boolean {
  const [slow, setSlow] = useState(false);
  useEffect(() => {
    if (!waiting) {
      setSlow(false);
      return;
    }
    const timer = window.setTimeout(() => setSlow(true), 4000);
    return () => window.clearTimeout(timer);
  }, [waiting]);
  return slow;
}
