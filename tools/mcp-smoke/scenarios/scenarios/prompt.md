Build the model from the "Scenarios, sensitivity and goal seek" tutorial's
first section, through the MCP tools (server "framework"). The document you
are connected to starts as a demo — delete its objects before you begin
(the "Orders" frame first, since it reads the demo's "Assumptions" block);
this build reuses the "Assumptions" name for its own container.

1. An "Assumptions" container holding four editable values: Price 120,
   Annual units 8000, Unit cost 70, Fixed costs 250000.
2. Two scenarios over those values: Upside (Price 125, Annual units 9500)
   and Downside (Price 115, Annual units 6500).
3. A "Plan" frame of twelve rows, one per month, with a Weight column
   holding 6, 6, 7, 8, 9, 9, 9, 9, 8, 9, 10, 10 (in that order), and
   calculated columns:
   ```text
   Units = `Annual units` * `Weight` / 100
   Revenue = `Units` * `Price`
   Cost = `Units` * `Unit cost`
   ```
4. A "Model" block with:
   ```text
   revenue = `Plan`.`Revenue`.sum()
   gross = revenue - `Plan`.`Cost`.sum()
   ebitda = gross - `Fixed costs`
   margin = ebitda / revenue
   ```

Then, without switching the whole document onto either scenario:

5. Add two more lines to the Model block that read what EBITDA would be
   under each scenario without activating it:
   ```text
   upside ebitda = under(`Upside`, ebitda)
   downside ebitda = under(`Downside`, ebitda)
   ```
6. Add a goal-seek line that finds the price hitting 300,000 EBITDA at the
   base volume:
   ```text
   target price = solve(ebitda == 300000, by=`Price`, within=[100, 200])
   ```

HARD REQUIREMENT: the document must still be on the Base scenario when you
finish — do not call anything that activates Upside or Downside. Save the
workbook.
