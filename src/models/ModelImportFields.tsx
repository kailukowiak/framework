export function ModelImportFields({ method, json, onJson, fileName, onPick }: {
  method: "xgboost" | "onnx"; json: string; onJson: (value: string) => void;
  fileName: string; onPick: () => void;
}) {
  return <>
    {method === "xgboost" ? <label>XGBoost model JSON<textarea aria-label="XGBoost model JSON" rows={5} value={json}
      spellCheck={false} placeholder={'Paste the result of save_model("model.json")'} onChange={event => onJson(event.target.value)} /></label>
      : <p className="model-mapping">Import a fitted sklearn linear or binary logistic pipeline with two numeric inputs and one categorical input. Its saved preprocessing is retained.</p>}
    <button className="secondary-action" onClick={onPick}>Choose model file…</button>
    {fileName && <p className="model-mapping" role="status">{fileName}</p>}
  </>;
}
