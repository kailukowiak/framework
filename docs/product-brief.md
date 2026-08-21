# Initial product brief

## Audience

The first user is a data analyst who understands dataframe concepts but repeatedly opens a spreadsheet for visual inspection, ad-hoc calculations, small corrections, comparisons, and presentation.

Financial modellers and Excel power users are a later audience. Their workflows will require deeper support for sequential calculations, scenarios, keyboard navigation, and formatting.

## Job to be done

When an analyst needs to explore or communicate structured data, FrameWork lets them import it, see it, calculate against it, and arrange the results visually without turning the work into fragile coordinate-based formulas or switching to a notebook.

## Initial promise

> Spreadsheet immediacy with dataframe structure.

## Acceptance scenario

An analyst can:

1. Paste an orders dataset onto a canvas.
2. Add a standalone tax-rate assumption.
3. calculate a `Total` column from `Quantity`, `Unit price`, and the tax rate.
4. See all rows recalculate after editing the assumption.
5. Override the calculation for one exceptional row and see it marked explicitly.
6. Rename `Quantity` to `Units` without breaking the formula.
7. Add a total or average summary.
8. Move the objects independently.
9. Undo changes and reopen the locally saved document.

## Not yet

FrameWork is not an Excel replacement. It intentionally excludes cell coordinates, XLSX compatibility, many-to-many and approximate joins, large-data execution, complete collaboration semantics, and sequential models until the fundamental interaction has been validated.
