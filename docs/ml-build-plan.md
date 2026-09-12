# ML build lanes and integration order

Execution breakdown of [MLConsensus.md](../MLConsensus.md), started after Kai
authorized subagent work. The initial batch builds evidence and an executable
contract; it does not ship model cards or announce working ML in FrameWork.

## First batch: independent work with explicit ownership

| Lane | Owner | Owned paths | Deliverable |
|---|---|---|---|
| Contract and integration | Astra (root) | `tools/ml-spikes/contract/`, this file | Typed serialized examples, shape validation, review of evidence and next integration gates |
| Statistical acceptance | statistics subagent | `tools/ml-spikes/statistics/`, `docs/ml-statistics-spike.md` | Pinned anofox harness, independent reference outputs, failure-case behavior |
| Imported workflows | imports subagent | `tools/ml-spikes/imports/`, `docs/ml-import-spike.md` | Reproducible sklearn/ONNX/XGBoost fixtures, parity evidence and observed graph/schema requirements |
| Actual XGBoost | XGBoost subagent | `tools/ml-spikes/xgboost/`, `docs/ml-xgboost-spike.md` | Real native fit/reload, local linkage evidence, remaining shipping-platform packaging checks |

Each lane uses an isolated dependency environment. No lane changes workspace
dependencies or the application's document format in this batch. Agents report
executed checks separately from documented claims and untested platforms. The
integration owner reviews the results and resolves cross-lane contract changes.

### Initial results

| Lane | Result | Gate still open |
|---|---|---|
| [Contract](../tools/ml-spikes/contract/README.md) | Four serialized examples; eight shape/identity tests pass; isolated Clippy passes with warnings denied | Incorporate observed import precision, missing sentinels, typed class labels, iteration policy, and data-dependent recipe output binding |
| [Statistics](ml-statistics-spike.md) | Representative OLS/HC3 and ordinary logistic reference parity; four tests pass | One explicitly ignored separation acceptance test fails when run; logistic intervals need a validated adapter; full HC3 covariance is not supplied by the tested public API |
| [Imports](ml-import-spike.md) | Two branched sklearn pipelines pass ONNX Runtime parity; three XGBoost objectives pass reload parity with the saved iteration policy | Implement native translation and tree execution; observed operator motifs exceed the historical short list |
| [XGBoost](ml-xgboost-spike.md) | Real native binary fit/reload and local dylib relocation pass | Prebuilt library requires macOS 15.0; clean production packaging, Windows/Linux, and signed app integration remain untested |

This batch supports continuing the build. It does not yet approve a shipping
logistic backend, ONNX tree runtime, or cross-platform XGBoost package. In
particular, a passing default statistics harness must never hide the named
separation release blocker.

Validation for this batch: contract tests (8 passed), statistical tests (4 passed,
1 named separation gate ignored by default and confirmed failing explicitly),
five Python import parity cases, and the native XGBoost fit/relocation smoke
checks. All three isolated Rust crates passed Clippy with warnings denied.
`npm run lint:rust` completed successfully with existing warnings in untouched
workspace code. No native UI e2e run applies to this isolated, non-product batch.

## First batch acceptance

The contract examples must serialize and validate, and reject representative
mapping mistakes. Backend harnesses must use real reference libraries; expected
numbers cannot come from the implementation being tested. A failing statistical
case remains a named blocker even if the surrounding harness passes by recording
that known limitation. An installed runtime passing parity is not proof that a
custom translator or tree-walk exists.

All artifacts in the contract examples are illustrative. Reference fixture data
in the numerical spikes is synthetic and generated independently. Neither kind
of fixture touches the user's workbooks or tutorial library.

## Next batch: resolve evidence before wiring cards

### Batch 2 completed — isolated implementations

Kai authorized this batch after the first three subagents completed. All three
agents have now finished their bounded assignments. Root reran their suites:
statistics 15 passed with one ignored raw-backend defect reproducer, ONNX 8 passed,
and XGBoost import 5 passed. These results validate the documented spike subsets,
not production application integration or general model-format compatibility.

| Owner | Assignment | Owned paths | Completion gate |
|---|---|---|---|
| Astra | Refine the common import semantics and review integration | `tools/ml-spikes/contract/`, this plan | Shared typed precision, missing sentinel, labels, scaler convention and round range, with validation tests |
| Statistics subagent | Implement guarded binary logistic adapter | `tools/ml-spikes/statistics/`, statistics report | Validated complete/quasi separation handling, explicit failure results, and normal-Wald interval parity |
| Imports subagent | Parse and execute the actual branched ONNX linear/logistic motifs natively | `tools/ml-spikes/onnx/`, existing import fixtures/generators and report | Source-protobuf-to-typed-plan parity plus strict unsupported/malformed-case rejection |
| XGBoost subagent | Parse numerical gbtree JSON and execute in Rust | `tools/ml-spikes/xgboost-import/`, `docs/ml-xgboost-import.md` | Regression/binary/multiclass parity, early-stopping policy, missing/boundary behavior and malformed-case rejection |

ONNX and XGBoost consumers share the contract crate by path dependency instead
of defining competing generic model types. Internal graph and tree types remain
owned by their implementation lanes. Root runs workspace lint once; each lane
runs its isolated tests and lint. Production application dependencies and public
operations remain outside this batch.

Clean Windows/Linux XGBoost packaging and the ONNX TreeEnsemble-versus-ORT
comparison remain separate outstanding checks. The current agents can make
useful local progress without pretending those targets have been exercised.

The logistic adapter now rejects the tested complete/quasi-separated cases and
provides reference-checked normal-Wald intervals. Its current 256-row/16-predictor
limits are spike bounds, not an acceptable implicit product limit. The ONNX
importer accepts one checked branched linear/logistic motif, not general sklearn
pipelines. Native XGBoost scoring covers the documented numerical gbtree subset.

Next recommended batch: integration review and a single production fitted-model
path. Finalize the contract and artifact boundary, then add model operations,
persistence/history, and a minimal dense card backed by real generated fixtures.
Prove import → map features → predict → upstream edit → save/reopen through native
e2e. Supported XGBoost import is a useful first vertical slice because it does not
depend on shipping the native training library. Keep native OLS fitting as the
next provider on that same path. Do not turn the small logistic spike limits or
exact ONNX motif into broad user-facing capability claims.

1. Reconcile the ordered recipe with actual imported graph semantics: feature
   width/order, missing sentinels, category behavior, and prediction policy.
2. Close statistical correctness blockers or explicitly limit the supported
   estimator/inference capabilities. Pinning anofox does not waive these gates.
3. Implement the narrow native import translator/tree executor and compare its
   supported ONNX tree cases with ORT. The first fixture batch does not settle
   that choice by itself.
4. Complete clean-machine and supported-platform XGBoost packaging evidence,
   then record the explicit ship/defer decision. A local toy fit is insufficient.

These can again be separate agents with disjoint modules, after the integration
owner publishes the refined contract. Do not launch UI work against changing
backend assumptions just to make all lanes look busy.

## Production implementation batches

### Production work authorized and underway

Kai asked to continue until the result is a finished product. The active lanes
are now: reusable `framework-ml` runtime (statistics agent), first-class core
model/operations/live frames (XGBoost agent), and dense authoring/cards/native e2e
(imports agent). Root owns desktop commands, MCP, dependency integration,
documentation, and cross-layer verification. Work is being integrated into the
application rather than left in the spike directories.

The first production slice includes native OLS/classical/HC3 and guarded binary
logistic fitting, explicit evaluation splits and training-derived baselines,
supported imported XGBoost inference, model-summary frames, and live prediction
frames. This is the first integration gate, not completion of the remaining
pipeline-import and broader-model requirements. No release/tag is authorized by
this build instruction, and none is part of this work.

Once the relevant gates pass, allocate work in this order:

1. **Core fitted model and artifacts.** Integrate validated types, input/feature
   mappings, native fitting, inference, retained diagnostics, and evaluation.
2. **Operations and persistence.** Add preparation/application/inversion, fitted
   revision lifecycle, save/reopen, collaboration artifact references, and MCP
   catalog parity. This depends on the core contract; it is not an independent
   competing definition of the operation model.
3. **Cards and authoring.** Build the compact model/recipe surface and stats
   frames from real generated fixtures. Keep estimator specification separate
   from engine identity and support multiple output descriptors.
4. **Product seam verification.** A dedicated integration lane exercises fit,
   live scoring, staleness, failed retraining, history, and persistence through
   the real native e2e harness. Add the user-facing changelog entry in this batch.

Pipeline import then uses the same cards and fitted-output path. Boosting and
additional statistics expand that path rather than building new model systems.

## Reporting rule

Finish each batch with what was actually built, commands that ran, concrete
failures, and the next dependency gate. Do not mark the full ML plan complete
when only a spike or contract is complete, and do not promote isolated harness
results into claims about the shipping app.

## Initial product scope decisions (2026-09-12)

The integrated initial catalog is native OLS (classical/HC3), binary logistic,
seeded random forests for regression/classification, mean confidence intervals,
Pearson correlation, and Welch differences of means. Imported inference includes
the tested numerical XGBoost JSON subset and the explicitly bounded sklearn ONNX
pipeline motif. Each addition uses the same operation/history/frame path.

**Native XGBoost training is deferred as an explicit scope cut.** The spike proves
local fit/reload/relocation, but its macOS dependency has a macOS 15 deployment
floor and required path repair. Clean supported-platform and signed-bundle evidence
is missing. Import inference ships independently of that native library. This
decision satisfies the requirement to decide after measuring; it does not claim
the native-training goal was delivered.

No ONNX Runtime is bundled for the supported linear/logistic graph subset. The
translator and executor match the supplied sklearn and ORT fixtures. An ONNX tree
executor-versus-ORT comparison is still open; results for linear graphs cannot
settle a tree-runtime packaging decision. Unsupported ONNX graphs are refused.

Large content-addressed model artifacts, full covariance/contrasts/prediction
intervals, a general user-authored fitted recipe editor, arbitrary sklearn graphs,
and additional model families are outside this initial product slice. Current
model payloads are typed, validated, bounded, and workbook-contained. These remain
tracked omissions from the wider consensus, not completed checkboxes.

## Integration handoff

The user-facing compatibility guide is [models.md](models.md); numerical API and
resource limits are in [ml-runtime.md](ml-runtime.md).

Input-column deletion is allowed. Models retain their fitted revision and show
missing-reference errors; prediction frames report missing scoring inputs. Tests
cover failed refitting, saving/reopening the broken workbook, undoing a recipe
repair back to its broken reference, and undoing the deletion to recover. This
implements Kai's correction that ordinary modeling mistakes should be editable
states rather than model-specific deletion prohibitions.

The numerical work and model validation live in focused modules. Existing large
object, identity, operation-definition, inversion, MCP and canvas files were
reduced through responsibility extraction. The canonical operation dispatchers
and frame-plan entrypoints retain small additional routing arms/hooks: these are
where the exhaustive public operation union and shared plan must connect to the
new modules. Moving the implementation out does not remove those integration
points. No lint exclusions or warning suppressions were introduced.

### Verification completed (2026-09-12)

- `cargo test --workspace`: 748 passed, three existing ignored tests.
- `npm test`: 904 passed across 143 files.
- App and e2e TypeScript checks passed; frontend lint passed with the existing
  45 warnings. Rust lint and formatting were checked without adding exclusions.
- A freshly built and signed debug e2e bundle passed all eight model workflow
  tests and all three existing Scratchwork baseline tests. The model workflow
  exercises fitting, live scoring, retained fits, import, deletion errors, undo,
  and save/reopen through the actual desktop application.

The final native run includes a desktop lock-order fix made after the workspace
test run: asynchronous operations release the document mutex before requesting
window focus or sending UI notifications. Otherwise a concurrent synchronous
document read could deadlock the window and worker threads. The model workflow
retains the document read that exposed this problem and passed against the rebuilt
bundle containing the fix.

Native e2e uses the documented embedded-driver focus and select-event
compensations. Visual density, native menus, and literal operating-system focus
remain manual checks in `npm run tauri dev`; automated integration passing does
not claim those manual checks were performed. No release was published.
