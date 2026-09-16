#[path = "cases/forest.rs"]
mod forest;
#[path = "cases/import.rs"]
mod import;
#[path = "cases/pipeline.rs"]
mod pipeline;
#[path = "cases/regression.rs"]
mod regression;
#[path = "cases/statistics.rs"]
mod statistics;
// Native training exists only where the XGBoost runtime is packaged
// (macOS, Windows and Linux); elsewhere `train` refuses by design, so these
// cases would fail on the refusal rather than on anything they check.
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
#[path = "cases/xgboost_training.rs"]
mod xgboost_training;
