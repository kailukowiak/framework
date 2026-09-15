Score a table with a fitted model in FrameWork and place the predictions
beside another frame's rows. Do the work through the FrameWork MCP server
and leave the finished model in the open document.

Hard requirements:

1. Create an editable source table with eight rows and two numeric columns,
   Spend and Sales:
   - Spend = 10, 20, 30, 40, 50, 60, 70, 80
   - Sales = 72, 88, 112, 131, 149, 172, 190, 211
2. Create a linear regression (OLS) model predicting Sales from Spend with
   no holdout, and fit it.
3. Create live predictions for the source table from the fitted model. The
   prediction frame carries the source columns with the model's output
   beside them; leave it live — do not materialize it.
4. Create a live frame from the source table and pair the prediction column
   into it as a new column named Predicted, reading the prediction frame
   live: not by materializing it and not by retyping the numbers.
5. Leave the source at eight rows. The verifier will add a ninth row and
   expects both the prediction frame and the paired column to grow with it.

Use semantic names where named tools support them. Inspect the operation
catalog when you need a model, a prediction frame, a linked frame, or a
vector-pairing step. Do not import helper files and do not paste a
precomputed answer table.
