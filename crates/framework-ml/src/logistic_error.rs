use crate::{MlError, logistic_backend::FitError, separation::DetectionError};
pub(crate) fn convert(error: FitError) -> MlError {
    match error {
        FitError::InvalidShape=>MlError::InvalidInput("Logistic regression requires matching predictor/target rows and more rows than parameters".into()),
        FitError::ResourceLimit=>MlError::ResourceLimit("Exact separation checking exceeds its current arithmetic-work budget; reduce training rows or predictors".into()),
        FitError::NonFiniteInput=>MlError::InvalidInput("Logistic regression requires finite numeric inputs".into()),
        FitError::InvalidLabels=>MlError::InvalidInput("Logistic regression requires target values 0 and 1, with both classes present".into()),
        FitError::InvalidSettings=>MlError::InvalidInput("Invalid logistic confidence or iteration settings".into()),
        FitError::Detection(DetectionError::RankDeficient)=>MlError::RankDeficient,
        FitError::Detection(DetectionError::SolverLimit)=>MlError::ResourceLimit("Separation checking reached its time budget; no model was fitted".into()),
        FitError::Detection(_)=>MlError::InconclusiveSeparation,
        FitError::Separated(_)=>MlError::Separation,
        FitError::NonConvergence=>MlError::NonConvergence,
        FitError::Backend(message)=>MlError::Numerical(message),
        FitError::UnavailableInference=>MlError::Numerical("The fitted coefficient uncertainty is numerically unavailable".into()),
    }
}
