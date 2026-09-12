import type { ComponentProps } from "react";
import { ModelWorkbench } from "./models/ModelWorkbench";
import { ParameterInputsProvider } from "./ParameterInputs";

/** Document-scoped authoring surfaces share the canonical operation handler. */
export function DocumentWorkbench(props: ComponentProps<typeof ModelWorkbench>) {
  return <ParameterInputsProvider document={props.document} onOperation={props.onOperation}>
    <ModelWorkbench {...props} />
  </ParameterInputsProvider>;
}
