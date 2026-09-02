# Human release review

This is the 20-minute human pass for the parts of FrameWork that automated
tests cannot honestly prove: native macOS menus and dialogs, operating-system
focus, the system clipboard, multi-window isolation, and whether a dense model
still reads like a spreadsheet.

Do not rebuild every tutorial for every change. Run the automated suite on
each branch, use the five-minute relevant section while developing, and run
this complete script for a release candidate or after changes to menus,
selection, focus, persistence, tutorials, or window management.

## Before starting

- [x] Record the commit, macOS version, and whether this is `npm run tauri dev`

      or an installed build.
- [ ] Open **Data Library → Tutorials & examples**. Choose **Create tutorials**

      if the tutorial set is absent. Choose **Reset tutorials**, then confirm
      **Replace all tutorial workbooks**, so checkpoints begin from known data.
- [ ] Keep this file beside the app. Mark each section **Pass**, **Fail**, or

      **Blocked**. A workaround is useful diagnostic information, but it is
      not a pass.

If a check fails, stop only that section. Capture the exact menu or gesture,
active workbook, expected and observed result, whether undo recovered it, and
a screenshot. Continue with an independent section when possible.

## 1. Native menu and window isolation — 4 minutes

This section must use the real macOS menu. The e2e app is deliberately
menu-less and cannot replace it.

- [ ] **1.1** From the tutorial library, open **Vectors, dates, and visual

      joins — Answer key**. Find the **Checks** scratchwork block and confirm it
      shows `6` and `137600`, with no formula error.
- [ ] **1.2** Choose **File → New Window**. In the new window, open

      **Month-over-month formulas by pointing — Start** from its Data Library.
- [ ] **1.3** Return to the vectors window. Choose **File → Open…** and select

      `Month-end close — Answer key` from `Documents/FrameWork Tutorials`.
- [ ] **1.4** Confirm only the active window changes workbook. The other window

      must remain on **Month-over-month formulas by pointing — Start**; it must
      not show a loading page, library, or the selected file.
- [ ] **1.5** Repeat step 1.3 with **⌘O**, choosing a different tutorial

      workbook. Confirm again that only the active window changes.
- [ ] **1.6** Close the second window with **⌘W**. Confirm the remaining

      workbook stays open and usable.

Pass when menu and shortcut commands each affect exactly the focused window.

## 2. Copy, cut, paste, right-click, and undo — 5 minutes

Use **Month-over-month formulas by pointing — Start**. Keep the app focused;
do not paste through a remote-control text helper.

- [ ] **2.1** Click the Revenue cell showing `142,000`. Press **⌘C**, click the

      Revenue cell showing `91,000`, then press **⌘V**. Confirm the target shows
      `142,000`. Press **⌘Z** and confirm it returns to `91,000`.
- [ ] **2.2** Click a Cost cell and press **⌘X**. Confirm the source becomes

      blank. Click a different Cost cell and press **⌘V**. Confirm the copied
      value appears. Undo until both cells are back to their original values.
- [ ] **2.3** First select one cell, then right-click a different Revenue cell.

      Choose **Copy** from the context menu, paste into a third Revenue cell,
      and confirm the value came from the cell that was right-clicked—not the
      earlier selection. Undo the paste.
- [ ] **2.4** Drag across a two-cell range, copy it, select a two-cell

      destination, and paste. Confirm shape and order are preserved. Undo.
- [ ] **2.5** Repeat one copy and paste using **Edit → Copy** and

      **Edit → Paste** instead of shortcuts.

Pass when every entry point uses the visible selection, edits once, and undo
restores the prior grid.

## 3. Tutorial semantics and live recomputation — 4 minutes

- [ ] **3.1** Reset and open **Month-over-month formulas by pointing — Start**.

- [ ] **3.2** In the **Checks** block enter:

  ```text
  Total revenue = `Monthly sales`.`Revenue`.sum()
  ```

- [ ] **3.3** Confirm the gutter shows `839000`. Edit the April Revenue cell

      from `142000` to `143000`; confirm the gutter changes to `840000`. Undo
      and confirm it returns to `839000`.

- [ ] **3.4** Open **Vectors, dates, and visual joins — Answer key**. Confirm

      **Checks** shows `6` and `137600`, **Scheduled launches** has six rows,
      and the **Scenario × Quarter** matrix shows `100.00` in its upper-left
      result cell.

- [ ] **3.5** Scan every visible scratchwork gutter and card for **Formula

      error**. There should be none.

Pass when formulas are live, the current answer key agrees with its written
checkpoints, and undo recomputes rather than leaving a stale answer.

## 4. Tabs, view state, and inspection — 3 minutes

- [ ] **4.1** Open **Month-end close — Answer key**.
- [ ] **4.2** On the tabbed analysis card, switch among **Actuals vs budget**,

      **Regional summary**, **Revenue by month**, and **Below budget**. Confirm
      the selected tab and the visible columns change together; no tab opens
      an empty card.
- [ ] **4.3** On **Actuals vs budget**, apply a visible filter or sort from the

      column menu. Confirm the grid changes and the view operation appears
      where the UI says it will be stored.
- [ ] **4.4** Open a numeric column profile. Confirm count and summary

      statistics render, then close and reopen it without changing the table.
- [ ] **4.5** Undo the filter or sort and confirm the original rows return.

Pass when tabs, inspection, and undo all describe the same visible data.

## 5. Human visual and focus pass — 3 minutes

Use the answer keys from sections 3 and 4 at normal window size and 100% zoom.

- [ ] **5.1** Click through a table cell, name field, formula editor, tab,

      scratchwork line, context menu, and back to the canvas. Confirm the focus
      ring follows the last gesture and typing never lands in a previously
      active editor.
- [ ] **5.2** Scan the canvas at a glance. Table names must be at least as

      prominent as their values; ordinary values must not use display-sized
      type; errors must appear beside the thing that failed rather than in a
      large alert card.
- [ ] **5.3** Resize one table card smaller and larger. Confirm columns remain

      usable and open-ended content is not trapped in a fixed-height card.
- [ ] **5.4** Open and cancel **File → Open…**. Confirm the current workbook

      returns unchanged and keyboard focus is usable immediately.

Pass when the app remains dense, legible, and predictable after focus and
native-dialog transitions.

## Result

```text
Build / commit:
macOS:
Reviewer:
Date:

1. Native menu and window isolation: Pass / Fail / Blocked
2. Clipboard and undo:               Pass / Fail / Blocked
3. Tutorial semantics:               Pass / Fail / Blocked
4. Tabs and inspection:              Pass / Fail / Blocked
5. Visual and focus:                  Pass / Fail / Blocked

Issue(s):
Lesson, section, and step:
Expected:
Observed:
Did undo recover it?
Screenshot or exact error:
```

## What automation covers before this review

- `npm test` covers React-only selection, context-menu, clipboard-target, and
  application-menu listener behavior.
- `cargo test -p framework-core --test integration tutorials::` loads every
  bundled Start and Answer key, evaluates every computed card and frame
  pipeline, rejects formula errors, and checks the named tutorial answers.
- `npm run test:e2e` drives the real debug app across tutorial reset, document
  load, grid editing, formula recomputation, undo, persistence, multiple
  windows, generators, joins, plots, and vector drag.

Those checks should be green first. This script does not duplicate them; it
reviews the operating-system and visual seams they cannot observe.
