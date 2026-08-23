import type { DocumentView, FrameObject, Operation } from "./types";
import type { VectorCombineState } from "../VectorCombinePrompt";

type Apply = (operation: Operation) => Promise<DocumentView>;

/**
 * VStack needs the dropped vector to become rows before the frame engine can
 * combine it. Keep that row source visible beside the result: the operation
 * stays inspectable instead of hiding a generated input in opaque plumbing.
 */
export async function combineVectorWithFrame(
  initial: DocumentView,
  state: VectorCombineState,
  apply: Apply
): Promise<DocumentView> {
  const target = frameById(initial, state.frameId);
  const targetView = initial.views.find((view) => view.id === state.viewId);
  if (!target || !targetView)
    throw new Error("The table receiving this vector is no longer available");
  if (target.columns.length !== 1)
    throw new Error("VStack begins with a one-column table");

  const helperName = nextObjectName(initial, `${state.vector.name} rows`);
  const beforeIds = new Set(initial.objects.map((object) => object.id));
  const next = await apply({
    type: "addGeneratorFrame",
    name: helperName,
    formula: state.vector.formula,
    // Union maps columns by name, once, when its step is prepared.
    columnName: target.columns[0].name,
    x: targetView.x + targetView.width + 36,
    y: targetView.y,
  });
  const helper = next.objects.find(
    (object): object is FrameObject =>
      object.kind === "frame" && !beforeIds.has(object.id)
  );
  if (!helper) throw new Error("The dropped vector did not create its row source");

  return apply({
    type: "setFramePipeline",
    frameId: target.id,
    steps: [{ kind: "union", frameId: helper.id }],
  });
}

function frameById(document: DocumentView, id: string): FrameObject | undefined {
  const object = document.objects.find((candidate) => candidate.id === id);
  return object?.kind === "frame" ? object : undefined;
}

function nextObjectName(document: DocumentView, stem: string): string {
  const names = new Set(document.objects.map((object) => object.name));
  if (!names.has(stem)) return stem;
  let suffix = 2;
  while (names.has(`${stem} ${suffix}`)) suffix += 1;
  return `${stem} ${suffix}`;
}
