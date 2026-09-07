import { chainSteps, chainFilterCount } from "../FrameGrid";
import type { ComputedFrame, RenderedFrameStep } from "./types";

/** What a frame's authored chain does to its source, in three words or fewer. */
export function frameTransformationLabels(computed: ComputedFrame): Array<string | null> {
  if (!computed.derivation) return [];
  const steps = chainSteps(computed);
  const filterCount = chainFilterCount(computed);
  const summarize = steps.find(
    (step): step is Extract<RenderedFrameStep, { kind: "summarize" }> =>
      step.kind === "summarize"
  );
  return [
    filterCount ? `${filterCount} filter${filterCount === 1 ? "" : "s"}` : null,
    steps.some((step) => step.kind === "join")
      ? "joined"
      : summarize?.aggregates.length
        ? summarize.groupKeys.length
          ? "grouped"
          : "total"
        : "linked",
    steps.some((step) => step.kind === "sort") ? "sorted" : null,
  ].filter(Boolean);
}


export function frameFilters(computed: ComputedFrame) {
  return {
    predicates: chainSteps(computed).flatMap((step) =>
      step.kind === "filter" ? step.predicates : []
    ),
    matchAll: true,
  };
}
