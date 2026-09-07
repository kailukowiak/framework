# Clean categories with a dictionary

About 5–10 minutes. Open **Dictionaries and value mapping — Start** from
**Data library → Tutorials and examples**. The answer key is a separate workbook.

## 1. Turn the small table into a dictionary

The **Expenses** table has two misspelled category entries and one category
that needs a shorter label. **Category fixes** lists the old and new labels.

Right-click **Key** in Category fixes and choose **Use as dictionary — key: Key**.
The table now says **dictionary**. Keys must be unique: one old label has one
replacement. This uses the same table; nothing is copied.

To start one from scratch in your own workbook, right-click empty canvas and
choose **Add dictionary here**, then type or paste entries under Key and Value.

## 2. Apply it to a column

Right-click **Category** in Expenses. Open **Map values…**, then choose
**Using Category fixes**. The new formula opens in **Wrangle** and applies live.

Both `Offce supplies` rows become `Office supplies`; `Travel & meals` becomes
`Travel`; `Software` stays unchanged. There are still four rows and the amounts
still total **200**. This is a transformation over the source, not a cell edit.
The same mapping action works with live and imported tables.

## 3. Change the rule once

Click a blank area of the canvas to leave the formula editor. In Category fixes,
edit the first **Value**, changing `Office supplies` to `Office costs`.
Both matching Expenses rows update. Press **⌘Z** to restore `Office supplies`.
You can edit a cell with a click followed by **F2**.

## 4. Look up one value

In **Checks**, keep the existing total line and add:

```text
label = lookup("Offce supplies", `Category fixes`.`Key`, `Category fixes`.`Value`)
missing = lookup("Software", `Category fixes`.`Key`, `Category fixes`.`Value`, "Not mapped")
```

The answers are **Office supplies** and **Not mapped**. Without a fallback,
an unmatched non-null lookup key shows a formula error. Map values keeps
unmatched entries by default. A blank replacement is a real null replacement;
it does not mean “skip this rule.”

## 5. Check the guardrail

Try changing the second Key to `Offce supplies`. The duplicate is refused and
both original rules remain. Each key can have only one replacement. Rules
match exact values; their row order does not affect the result.

The **Answer key** contains the completed mapping and both lookup formulas.
Its dictionary remains editable, so the same change-and-undo experiment works
there too. Filtering the dictionary's display never disables a mapping.
