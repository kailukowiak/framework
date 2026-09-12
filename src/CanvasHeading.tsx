import type { DocumentView } from "./lib/types";

export function CanvasHeading({ document }: { document: DocumentView }) {
  return <div className="canvas-heading">
    <span>ANALYSIS CANVAS</span>
    <h1>{document.name}</h1>
    <p>{document.objects.length === 0
      ? "Nothing here yet. Press ⌘J to start writing, open a document or a sample from the Data library, import a file, or add a frame."
      : `${document.objects.length} object${document.objects.length === 1 ? "" : "s"} · ${document.views.length} window${document.views.length === 1 ? "" : "s"}`}
    </p>
  </div>;
}
