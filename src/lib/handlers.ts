import type { Operation } from "./types";

/**
 * How a card asks for the document to change.
 *
 * This lives here rather than beside the components because both the shell
 * and the editors split out of it need it, and a type owned by either one
 * would have the other importing back into its own parent. It is not a
 * grid type or a pipeline type -- it is the one call every card makes.
 */
export type OperationHandler = (
  operation: Operation,
  options?: { inlineError?: boolean }
) => Promise<string | null>;
