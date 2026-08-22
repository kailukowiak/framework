import { Download, X } from "lucide-react";
import { useEffect } from "react";
import { Markdown } from "./Markdown";
import type { UpdateProgress, UpdateStatus } from "./hooks/useUpdateCheck";

function formatBytes(bytes: number): string {
  return bytes >= 1_048_576
    ? `${(bytes / 1_048_576).toFixed(1)} MB`
    : `${Math.round(bytes / 1024)} KB`;
}

function progressLabel({ received, total }: UpdateProgress): string {
  return total === null
    ? `Downloading — ${formatBytes(received)}`
    : `Downloading — ${formatBytes(received)} of ${formatBytes(total)}`;
}

/**
 * Offers a release that already exists rather than announcing one. The whole
 * card is three lines and three buttons on purpose: an update prompt is
 * interruption, and interruption that spends a screen explaining itself is
 * worse than the version skew it is trying to fix.
 *
 * Skip is a real answer, not a softer Later. Later means ask me next time;
 * Skip means stop asking about this version, and is remembered in
 * lib/updates.ts. Choosing Check for Updates by hand overrides both, because
 * asking is unambiguous about wanting an answer.
 */
export function UpdateDialog({
  status,
  progress,
  onInstall,
  onSkip,
  onDismiss,
}: {
  status: UpdateStatus;
  progress: UpdateProgress | null;
  onInstall: () => void;
  onSkip: () => void;
  onDismiss: () => void;
}) {
  const busy = status.kind === "installing";
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onDismiss();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onDismiss, busy]);

  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget && !busy) onDismiss();
      }}
    >
      <div className="insert-dialog update-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">UPDATE</span>
            <h2>
              <Download size={16} />
              {status.kind === "available" || status.kind === "installing"
                ? `FrameWork ${status.version}`
                : "Check for updates"}
            </h2>
          </div>
          {!busy && (
            <button className="icon-button" onClick={onDismiss} aria-label="Close">
              <X size={18} />
            </button>
          )}
        </div>

        {status.kind === "checking" && <p>Checking…</p>}

        {status.kind === "up-to-date" && <p>FrameWork is up to date.</p>}

        {status.kind === "unsupported" && (
          <p>
            This copy was installed by your package manager, so it updates the
            same way everything else on your system does — through{" "}
            <code>apt</code>, <code>dnf</code>, or your software centre. Nothing
            to do here.
          </p>
        )}

        {status.kind === "failed" && (
          <p>
            Could not check for updates. {status.message}
          </p>
        )}

        {(status.kind === "available" || status.kind === "installing") && (
          <>
            <p>Installing restarts FrameWork; open documents are saved.</p>
            {status.notes && (
              <div className="update-notes">
                <Markdown source={status.notes} />
              </div>
            )}
            {progress && <p className="update-progress">{progressLabel(progress)}</p>}
          </>
        )}

        <div className="dialog-actions">
          {status.kind === "available" && (
            <>
              <button className="secondary-action" onClick={onSkip}>
                Skip this version
              </button>
              <button className="secondary-action" onClick={onDismiss}>
                Later
              </button>
              <button className="primary-action" onClick={onInstall}>
                Install and restart
              </button>
            </>
          )}
          {status.kind === "installing" && (
            <button className="primary-action" disabled>
              Installing…
            </button>
          )}
          {status.kind !== "available" && status.kind !== "installing" && (
            <button className="primary-action" onClick={onDismiss}>
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
