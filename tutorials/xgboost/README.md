# Classify Iris species with XGBoost

A compact lesson in fitting a multiclass booster, checking it on unseen rows,
and scoring new measurements. The 150-row Iris dataset is bundled with
scikit-learn; this workbook keeps rows 49, 99, and 149 aside as unlabeled
examples and uses the other 147 rows for model fitting and its random holdout.

Species codes are **0 = setosa**, **1 = versicolor**, and **2 = virginica**.
They are numeric class labels, not measurements with meaningful distance.

## 1. Create the model

Right-click **Iris training** and choose **Create model…**. Name it
**Iris classifier**, choose **Predict multiple categories · XGBoost**, and
choose **Species code** as the target. All four measurement columns become
predictors automatically. To use fewer predictors, first select the columns you
want in a derived frame through Wrangle/formulas, then model that frame.

Keep the 20% holdout and default booster settings. Choose **Create model**,
then **Fit model** on the new card. Fitting is explicit: later training edits
mark the fit stale but do not silently replace it.

## 2. Check unseen performance

Read **Holdout evaluation metrics**. Accuracy is the share of held-out rows
classified correctly. Compare it with baseline accuracy, which always chooses
the most common training class. Log loss also uses predicted probabilities;
lower is better and confident wrong answers cost more.

This small random holdout demonstrates the workflow. It is not a broad claim
about botanical identification in other places, seasons, or measurement setups.

## 3. Score the three new flowers

Choose **Predictions…** on the model card. Name the result
**Species predictions**, select **New flowers**, and map the four measurement
columns in the same order. Choose **Create predictions**. The prediction frame
shows a class code and one probability per species; the scoring frame needs no
target column.

## 4. Try a what-if and preserve it

Change the first **Petal length** in **New flowers** from **1.4** to **5.9**.
Its probabilities should update from the saved model without refitting; the
winning class code may or may not change. Press
**⌘Z** on Mac or **Ctrl+Z** on Windows to restore the original measurement.

Save, close, and reopen the workbook. The fitted trees and live prediction
frame survive. The **Answer key** already contains the completed model and
predictions.

## Data source

The source folder pins scikit-learn **1.7.2** and verifies the [official bundled
`iris.csv`](https://github.com/scikit-learn/scikit-learn/blob/1.7.2/sklearn/datasets/data/iris.csv)
checksum before exporting these rows. Iris is the classic R. A. Fisher dataset;
see scikit-learn's [dataset description](https://github.com/scikit-learn/scikit-learn/blob/1.7.2/sklearn/datasets/descr/iris.rst)
for its full provenance and license notes.
