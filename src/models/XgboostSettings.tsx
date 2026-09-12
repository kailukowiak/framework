import type { Settings } from "../lib/bindings/Settings";
import type { Method } from "../lib/bindings/Method";

export type ModelChoice = Method | "onnx" | "xgboostRegression" | "xgboostBinary" | "xgboostMulticlass";
export const defaultXgboost: Settings = {
  objective: "regression", rounds: 100, maxDepth: 6, learningRate: 0.1,
  minChildWeight: 1, subsample: 1, columnSubsample: 1, l2Regularization: 1, seed: 0, threads: 1,
};
export function nativeMethod(method: Exclude<ModelChoice, "onnx">): Method {
  if (method === "xgboostRegression" || method === "xgboostBinary" || method === "xgboostMulticlass") return "xgboost";
  return method;
}
export function xgboostObjective(method: string): Settings["objective"] | null {
  if (method === "xgboostRegression") return "regression";
  if (method === "xgboostBinary") return "binary";
  return method === "xgboostMulticlass" ? "multiclass" : null;
}
export function xgboostChoice(settings: Settings): ModelChoice {
  return settings.objective === "regression" ? "xgboostRegression"
    : settings.objective === "binary" ? "xgboostBinary" : "xgboostMulticlass";
}

const inputValue = (value: number) => Number.isNaN(value) ? "" : value;

export function XgboostSettings({ settings, onChange }: { settings: Settings; onChange: (value: Settings) => void }) {
  return <details><summary>XGBoost settings</summary><div className="model-settings-row">
    <label>Boosting rounds<input type="number" min="1" max="1000" value={inputValue(settings.rounds)}
      onChange={event => onChange({ ...settings, rounds: event.target.valueAsNumber })} /></label>
    <label>Maximum depth<input type="number" min="1" max="16" value={inputValue(settings.maxDepth)}
      onChange={event => onChange({ ...settings, maxDepth: event.target.valueAsNumber })} /></label>
    <label>Learning rate<input type="number" min="0.001" max="1" step="0.01" value={inputValue(settings.learningRate)}
      onChange={event => onChange({ ...settings, learningRate: event.target.valueAsNumber })} /></label>
    <label>Minimum child weight<input type="number" min="0" max="1000000" value={inputValue(settings.minChildWeight)}
      onChange={event => onChange({ ...settings, minChildWeight: event.target.valueAsNumber })} /></label>
    <label>Row sample fraction<input type="number" min="0.01" max="1" step="0.1" value={inputValue(settings.subsample)}
      onChange={event => onChange({ ...settings, subsample: event.target.valueAsNumber })} /></label>
    <label>Column sample fraction<input type="number" min="0.01" max="1" step="0.1" value={inputValue(settings.columnSubsample)}
      onChange={event => onChange({ ...settings, columnSubsample: event.target.valueAsNumber })} /></label>
    <label>L2 regularization<input type="number" min="0" max="1000000" step="0.1" value={inputValue(settings.l2Regularization)}
      onChange={event => onChange({ ...settings, l2Regularization: event.target.valueAsNumber })} /></label>
    <label>Training seed<input type="number" min="0" value={inputValue(settings.seed)}
      onChange={event => onChange({ ...settings, seed: event.target.valueAsNumber })} /></label>
    <label>Threads<input type="number" min="1" max="16" value={inputValue(settings.threads)}
      onChange={event => onChange({ ...settings, threads: event.target.valueAsNumber })} /></label>
  </div></details>;
}
