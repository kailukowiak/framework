import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import type { DataObject, DocumentView } from "../lib/types";
import blankJson from "./fixtures/blank.json";
import salesBeforeJson from "./fixtures/sales-before-formula.json";
import salesWithJson from "./fixtures/sales-with-formula.json";

// ---------------------------------------------------------------------------
// Support for mounted interaction tests.
//
// The rule that keeps this tier from growing back into the deleted 2.5k-line
// preview engine: nothing here computes. Fixtures are DocumentViews generated
// by framework-core (`cargo run -p framework-core --example
// generate_ui_fixtures`), and the invoke mock only replays what it is
// explicitly handed. A test that needs the answer to *change* after an
// operation is asking an engine question — that test belongs in Rust or in
// an e2e workflow spec, not here.
// ---------------------------------------------------------------------------

export const fixtures = {
  blank: blankJson as unknown as DocumentView,
  salesBeforeFormula: salesBeforeJson as unknown as DocumentView,
  salesWithFormula: salesWithJson as unknown as DocumentView,
};

/** Fixture ids are minted on regeneration, so tests select by name. */
export function objectNamed<K extends DataObject["kind"]>(
  view: DocumentView,
  kind: K,
  name: string
): Extract<DataObject, { kind: K }> {
  const found = view.objects.find(
    (object) => object.kind === kind && object.name === name
  );
  if (!found) throw new Error(`fixture has no ${kind} named ${name}`);
  return found as Extract<DataObject, { kind: K }>;
}

/**
 * Routes Tauri invoke to explicit answers, and refuses everything else by
 * name. The refusal is the guardrail: an unmocked command is a design
 * question, not a gap to fill with invented data.
 */
export function serveInvoke(
  answers: Record<string, (args: unknown) => unknown>
): void {
  mockIPC((command, args) => {
    const answer = answers[command];
    if (!answer) {
      throw new Error(
        `Unmocked command "${command}". Hand it an explicit answer — and if ` +
          `that answer would have to change as the document changes, this ` +
          `test belongs in cargo test or an e2e spec, not here.`
      );
    }
    return Promise.resolve(answer(args));
  });
}

export { clearMocks };
