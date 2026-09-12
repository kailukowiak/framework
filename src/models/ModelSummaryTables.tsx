import type { ModelSummary } from "../lib/bindings/ModelSummary";
import type { Metrics } from "../lib/bindings/Metrics";

function number(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  const magnitude = Math.abs(value);
  if (magnitude > 0 && (magnitude < 0.0001 || magnitude >= 10_000_000)) return value.toExponential(4);
  return new Intl.NumberFormat(undefined, { maximumSignificantDigits: 5 }).format(value);
}

export function CoefficientTable({ summary }: { summary: ModelSummary }) {
  if (!summary.coefficients.length) return null;
  const confidence = summary.confidenceLevel == null ? "" : `${number(summary.confidenceLevel * 100)}% `;
  const covariance = summary.covariance === "hc3" ? "HC3 standard errors"
    : summary.covariance === "classical" ? "Classical standard errors" : null;
  return <table className="model-results" aria-label="Model coefficients">
    <caption>Coefficients{covariance && ` · ${covariance}`}</caption>
    <thead><tr><th>Term</th><th>Estimate</th><th>SE</th><th>Statistic</th><th>p</th>
      <th>{confidence}CI low</th><th>CI high</th></tr></thead>
    <tbody>{summary.coefficients.map((coefficient, i) => <tr key={`${coefficient.term}:${i}`}>
      <th scope="row">{coefficient.term}</th><td>{number(coefficient.estimate)}</td>
      <td>{number(coefficient.standardError)}</td><td>{coefficient.statisticType} {number(coefficient.statistic)}</td>
      <td>{number(coefficient.pValue)}</td><td>{number(coefficient.confidenceLower)}</td><td>{number(coefficient.confidenceUpper)}</td>
    </tr>)}</tbody>
  </table>;
}

export function MetricsTable({ metrics, label }: { metrics: Metrics; label: string }) {
  if (!metrics.rows) return null;
  const rows: Array<[string, number | null | undefined, number | null | undefined]> = [
    ["RMSE", metrics.rmse, metrics.baselineRmse], ["MAE", metrics.mae, null],
    ["R²", metrics.rSquared, null], ["Accuracy", metrics.accuracy, metrics.baselineAccuracy],
    ["Log loss", metrics.logLoss, metrics.baselineLogLoss],
  ];
  return <table className="model-results" aria-label={`${label} metrics`}>
    <caption>{label} · {metrics.rows} rows</caption>
    <thead><tr><th>Metric</th><th>Model</th><th>Baseline</th></tr></thead>
    <tbody>{rows.filter(([, value]) => value != null).map(([name, value, baseline]) =>
      <tr key={name}><th scope="row">{name}</th><td>{number(value)}</td><td>{number(baseline)}</td></tr>)}</tbody>
  </table>;
}
