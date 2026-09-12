import { useEffect, type ReactNode } from "react";
import { X } from "lucide-react";

export function ModelDialogShell({ title, busy, error, onClose, children }: {
  title: string; busy: boolean; error: string | null; onClose: () => void; children: ReactNode;
}) {
  useEffect(() => {
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onClose();
    };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [busy, onClose]);
  return <div className="model-dialog-backdrop" onPointerDown={event => event.stopPropagation()}>
    <section className="model-dialog" role="dialog" aria-modal="true" aria-label={title}>
      <header><h2>{title}</h2><button className="icon-button" aria-label="Close model editor"
        disabled={busy} onClick={onClose}><X size={16} /></button></header>
      <fieldset disabled={busy}>{children}</fieldset>
      {error && <p className="model-error" role="alert">{error}</p>}
    </section>
  </div>;
}
