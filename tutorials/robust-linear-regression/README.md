# Robust standard errors with diabetes progression

This lesson fits ordinary least squares to scikit-learn's diabetes dataset using
raw body mass index and blood pressure as predictors. The target is a quantitative
measure of disease progression one year after baseline. The first 439 observations
form **Diabetes training**; the last three are separate unlabeled scoring rows.

This historical clinical dataset is for teaching the model workflow. It does not
support diagnosis, treatment, causal claims, or use on an individual patient.

## 1. Fit OLS with HC3 uncertainty

Right-click **Diabetes training** and choose **Create model…**. Name the model
**Progression · HC3 OLS**, choose **Predict a number · Linear regression (OLS)**,
and choose **Progression** as the target. The other columns, **BMI** and
**Blood pressure**, are used automatically. To use fewer predictors, first select
the columns you want in a derived frame through Wrangle/formulas, then model that frame.

Choose **Robust (HC3)** standard errors, keep 95% confidence and the 20% holdout,
then choose **Create model** and **Fit model**.

HC3 changes coefficient standard errors, tests, and confidence intervals when
error variance is uneven. The fitted line is still ordinary least squares. HC3
does not make the coefficients resistant to outliers or influential observations.

## 2. Read the holdout before the coefficients

Under **Holdout evaluation metrics**, compare RMSE with baseline RMSE. Both use
rows the fit did not train on; the baseline predicts the training mean for every
row. Lower error is better. R² describes improvement over a constant prediction,
and can be below zero on unseen data.

Then inspect the coefficient table. Estimates are changes in predicted progression
per one raw-unit increase while holding the other predictor fixed. Confidence
intervals express sampling uncertainty under the model assumptions; they are not
prediction intervals for patients and do not establish causation.

## 3. Score new rows

Choose **Predictions…** on the model card. Name the result
**Progression predictions**, select **New patients**, and map **BMI** followed by
**Blood pressure**. Choose **Create predictions**. The new frame carries the
**New patients** columns with the prediction beside them; it uses the saved fit
and does not need known progression values.

## 4. Try a what-if, undo, and reopen

Change the first BMI in **New patients** from **24.9** to **30**. Its prediction
updates immediately; the other rows do not change and the model is not retrained.
Press **⌘Z** on Mac or **Ctrl+Z** on Windows to restore 24.9.

Save, close, and reopen the workbook. The fitted coefficients and prediction frame
survive. The **Answer key** contains the completed fit and scoring output.

## Data source

The source folder pins scikit-learn **1.7.2**, verifies both [official bundled
dataset files](https://github.com/scikit-learn/scikit-learn/tree/1.7.2/sklearn/datasets/data),
and exports raw BMI, raw average blood pressure, and progression. Scikit-learn's
[dataset description](https://github.com/scikit-learn/scikit-learn/blob/1.7.2/sklearn/datasets/descr/diabetes.rst)
records the 442 observations and the original study provenance.
