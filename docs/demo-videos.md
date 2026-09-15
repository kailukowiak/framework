# Demo videos

Four recordings that get people interested in FrameWork, each with a short
cut. The long versions are trimmed tutorials: same workbooks, same gestures,
but the checkpoints, explanations and "taught properly in" pointers are cut
and the narration says one sentence per idea. The short cuts are edited from
the same takes, not recorded separately.

Priority order is 1, 2, 4, 3. Videos 1 and 2 are the ones to ship first;
video 4 (Scratchwork) is short to record and goes third.

## Ground rules for every recording

- **Record from the tutorial Start workbook** after **Reset tutorials**, so the
  starting state is identical every take and matches what a viewer who
  installs the app will see. The one exception is video 2, which uses
  `demo-data/` (regenerate with `uv run examples/generate_accounting_demo.py`).
- **Record each section as its own take**, ending on the checkpoint state. The
  short cut is assembled from the same takes at faster pace, so a clean cut
  point at every section boundary matters more than a flawless single run.
- **Window size**: 1920×1080 logical points on a Retina display, giving a
  3840×2160 capture that downscales cleanly to 1080p and stays readable in a
  vertical crop for the shorts. Hide the Dock and menu bar extras. Set the
  canvas zoom so a six-row table fills about a third of the frame height.
- **Stage the clipboard** before a paste take. The paste block from the tour is
  the first thing on screen in video 1, so have it copied before you hit
  record.
- **Cursor**: turn on a cursor highlight for the drag-to-join and header-click
  gestures. Those are the two moments a viewer has to see the pointer.
- **Narration** is recorded separately over the cut, not live. It keeps the
  takes short and lets the short cut reuse footage with different words.
- **The short cut has no narration.** Captions only, one line per beat, and a
  title card for the last two seconds with the download link. It has to work
  muted in a feed.
- **Every number said aloud must match the checkpoint in the README.** If a
  number differs, that is a smoke-test finding, not a script edit.

## Video 1 — The FrameWork tour (6–8 min)

Source: `tutorials/grand-tour/README.md`. This is the general-audience video
and the one to put on the README and the release page.

**Cold open** (section 0 of the tour, 20 s): the finished workbook. Switch the
Scenario menu Base → Upside → Downside and watch Forecast and `Forecast total`
move together. Narration: "Three answers, nothing copied, nothing to put back
by hand. Here is how this workbook gets built."

Then the Start workbook, in tour order, with these cuts:

| Take | Tour section | Keep | Cut |
|---|---|---|---|
| 1 | 1 Paste a table | paste, note types arrive as numbers | the "why no convert-to-table" paragraph |
| 2 | 3 One formula, one column | `Profit = Revenue - Cost`, drag the fill square and it opens the formula instead | the three-column format step |
| 3 | 4 Declare the order | click the sort, open Wrangle and show Sort as a step | `.shift(1)` explanation |
| 4 | 5 Scratchwork | ⌘J, type `Total revenue = `, click the header, `.sum()` → `839000`; then Window → Scratchwork Window side by side | undo demonstration |
| 5 | 6 Drag a header to join | drag Budget onto Month, diagnostics 6/0/0, Mark unique, Bring columns over, `Variance` column | Pin columns, Total variance line |
| 6 | 7 Branch and summarize | + Table view, Summarize by Region, show the source tab still has six rows | the reconciliation paragraph |
| 7 | 8 What if | Assumptions container, Growth 1.08, Forecast column, add Upside and Downside, switch | the rounding remark |
| 8 | 9 Chart | + Plot, Bar, Month × Revenue by Region; edit a Revenue cell and the bar moves | order explanation |
| 9 | Finish line | undo the join and watch Variance, summary and chart go with it; redo | Export to Excel (its own short, see below) |

Section 2 (Find, Quick Commands, Reference) is cut from the long video. It
does not read on video, and the palettes get their own 30 s short if wanted.

**Short cut 1a — "One formula, one column" (60 s).** Takes 1, 2, 3, 5, 7 at
double pace:

| s | Beat | Caption |
|---|---|---|
| 0–8 | paste the block, table appears with typed columns | Paste. Columns have names and types. |
| 8–18 | `Profit = Revenue - Cost`, six values fill | One formula. The whole column. |
| 18–24 | drag the fill square, formula opens instead | There is nothing to fill down. |
| 24–36 | drag Budget onto Month, 6 matched, Bring columns over | Join by dragging a header. |
| 36–50 | Growth value, Upside, Downside, switch the scenario | Three scenarios. Nothing copied. |
| 50–58 | undo the join; Variance, summary and chart vanish; redo | Undo travels with the model. |
| 58–60 | title card | FrameWork. Free download. |

**Short cut 1b — "What if" (30 s).** Take 7 only, plus the cold open. Value,
column, two scenarios, switch three times. Caption: "A scenario is what
differs. Everything that reads it moves."

## Video 2 — A million rows, nine things wrong (5–7 min)

Source: `docs/demo-runbook.md`. This is the video for accountants, auditors
and FP&A, and the one that shows scale. Use `general_ledger.parquet`; fall
back to `general_ledger_2025q4.csv` only if the demo machine stutters, and
then skip Act 5's fee findings.

Keep Acts 1, 2, 5 and 6. Cut Acts 3, 4 and 7 from the long video; Act 4
(anti-join exceptions, accounting format at $K) belongs in video 3.

| Take | Act | Beat | The reveal |
|---|---|---|---|
| 1 | 1 | Open file… → parquet, 1,179,288 lines, scroll hard | header stays, no pause |
| 2 | 2 | group by `je_id`, `imbalance` aggregate, child filter `!= 0` | three rows: +0.01, −0.01, +100.00 out of 304,392 entries |
| 3 | 5 | import bank statement, inner join on reference, `difference` column, filter `!= 0` | `CHQ-6584`, 54.00, divisible by 9 |
| 4 | 5 | anti-join bank → GL | `SERVICE CHG ADJ` 347.50 nobody booked |
| 5 | 6 | branch to Auditor filter, three predicates | `JE-MGMT-1/2/3`, round dollars, year end, one user |

Narration for take 2 is the thesis of the video: "In a spreadsheet this is a
pivot over a million rows and a scan by eye. Here it is a group, a filter, and
three rows, and the filter is a visible step you can read back."

**Short cut 2a — "Three rows" (45 s).** Takes 1 and 2. Import, row count,
group, filter, three rows. Captions: "1.18 million journal lines." / "Group
by entry. Debits minus credits." / "Filter to anything that isn't zero." /
"Three. Found in seconds."

**Short cut 2b — "The auditor's filter" (30 s).** Take 5 only. Three
predicates typed one after another, the table narrowing each time, ending on
the three management accruals. Caption on the last frame: "Manual. Round
thousands. One user. Year end."

## Video 3 — Month-end close (6–8 min)

Source: `tutorials/month-end-close/README.md`, with Act 4 of the demo runbook
for the exception views if the tutorial's own exception step is thinner.
This is the video for the Excel power user who is not yet convinced, so the
gestures to keep are the ones with an Excel analogue that FrameWork does
differently:

1. **Join actuals to budget** on a marked-unique key. Say the word VLOOKUP
   once, then never again.
2. **Variance columns**, then the **accounting format at $K** with paren
   negatives and the badge in the header. Formatting changes how numbers read,
   not what they are.
3. **Exceptions as anti-joins**: budgeted and never spent, spent and never
   budgeted. Each is its own table with cords back to both parents.
4. **Control totals in a block** that stay live. Change an actual, the control
   moves, the exception table gains or loses a row.
5. **Export to Excel…** with the explanation sheet checked. "People who need a
   spreadsheet get a spreadsheet; you keep the model."

**Short cut 3a — "Where did the budget go" (60 s).** Join, variance, format,
one anti-join landing on the department that spent nothing. Captions only.

**Short cut 3b — "Export" (30 s).** Export to Excel, open the result, flip to
the explanation sheet. Caption: "The workbook they asked for. The model you
keep."

## Video 4 — Ad hoc: the back of the envelope (4–5 min)

Source: section 5 of the tour, the variable-controls section of
`docs/formula-function-catalog.md`, and the optional Scratchwork moment in
`docs/demo-runbook.md`. Recorded on the finished tour workbook, so every
line has real tables to point at. This is the video for the analyst who keeps
a spreadsheet open "just to check something", and the thesis is that the
check lives in the document and stays live.

### The sheet it replaces

The video opens on the spreadsheet everyone has: a tab called *Scratch* with
two labelled inputs, six rows pasted from somewhere, and five formulas off to
the right. `docs/demo-assets/scratch.xlsx` is that sheet; rebuild it with
`uv run docs/demo-assets/make_scratch_xlsx.py`. It uses the same six months
as the tour, so every answer ties to the tour workbook.

```text
     A          B        C          E            F
1    Growth     1.08
2    Tax rate   26%
4    Month      Revenue  Region     Total        =SUM(B5:B10)                 839,000
5    2026-01    118,000  East       East total   =SUMIF(C5:C9,"East",B5:B9)   405,000
6    2026-02    124,000  West       East share   =F5/F4                       48.3%
7    2026-03    136,000  East       Forecast     =ROUND(F4*B1,0)              906,120
8    2026-04    142,000  West       After tax    =F7*(1-B2)                   670,529
9    2026-05    151,000  East
10   2026-06    168,000  East
```

Two things are wrong with it on purpose, and both are the ordinary kind:

- **The SUMIF range stops at row 9.** June was pasted in after the formula was
  written. `SUM` was extended, `SUMIF` was not, so East total reads 405,000
  instead of 573,000 and everything downstream of it is quietly wrong.
- **`B1` and `B2` are the names of the inputs** as far as the formulas are
  concerned. The labels in column A are decoration.

On screen, click into F5 and let the range highlight show the gap. Then close
Excel. Do not fix it.

### The same five lines in Scratchwork

On the finished tour workbook, ⌘J and type these, pointing at headers rather
than typing the references:

```text
Growth = 1.08
Tax rate = 0.26
Total revenue = `Monthly sales`.`Revenue`.sum()
East total = `Monthly sales`.`Revenue`.filter(`Region` == "East").sum()
East share = East total / Total revenue
Forecast = (Total revenue * Growth).round()
After tax = Forecast * (1 - Tax rate)
```

Gutter answers: `839000`, `573000`, `0.6829…`, `906120`, `670528.8`.

The lines say what the sheet meant. There is no range to extend because the
column is the range; the inputs are referred to by name because the name is
the thing; and adding a July row to `Monthly sales` moves all five answers.
Do that on camera: type a seventh row into the append row and watch the
gutter.

A tiny vector, for the second half of the video, is one Variable holding a
list, the same way the vectors lesson writes one:

```text
Growth cases = [1.05, 1.08, 1.15]
```

which the slider take can replace with a control instead.

| Take | Beat | The point |
|---|---|---|
| 0 | `scratch.xlsx`: click F5, show the SUMIF range stopping a row short of June; close Excel | this is the sheet, and this is the bug it always has |
| 1 | ⌘J from anywhere; type `Total revenue = `, click the Revenue header, add `.sum()` → `839000` | a question is one line, and you point at columns instead of typing references |
| 2 | second line: `East revenue = \`Monthly sales\`.\`Revenue\`.filter(\`Region\` == "East").sum()` → `573000` | the SUMIF idea is one composable expression, not a parallel family of functions |
| 3 | the remaining lines from the block above; then type a July row into the append row and every answer moves; undo | lines read each other and read the tables; there is no range to extend |
| 4 | fourth line: `Target = slider(start=0.5, stop=0.8, step=0.01, value=0.68)`, then `Gap = East share - Target`; drag the slider | an assumption gets a control by writing one in; one drag is one undoable edit |
| 5 | while editing a line, ⌘K opens the Reference on formulas; find `.filter` | never more than one key from the name of the thing |
| 6 | Window → Scratchwork Window; put it beside the canvas; click a header in the main window and the reference lands in the pop-out line; close it, the block stays | the scratchpad is a second view of the same block, not a copy |

Narration for take 3 is the thesis: "A spreadsheet check is a number you
typed once. These are questions, and they keep answering."

**Short cut 4a — "Just checking" (60 s).** Takes 0 to 3 at pace: the SUMIF
range stopping short, then ⌘J, point at a column, `.sum()`; the conditional
total reading 573,000 where the sheet said 405,000; add a July row and every
answer moves. Captions: "Your scratch tab. The range stopped at row 9." /
"⌘J. Point at a column." / "A conditional total is one expression. It reads
the whole column." / "Add a row. Every answer follows."

**Short cut 4b — "Slider" (30 s).** Take 4 only. Type the slider line, drag
it, watch `Gap` change sign. Caption: "Write the control into the formula."

## Scratchwork inside video 1

Take 4 of the tour video grows to carry takes 1 and 6 of video 4: the
pointed `.sum()` line and the pop-out window. The conditional total, the
slider and ⌘K stay in video 4 so the tour keeps moving.

## Suggested short-cut set for launch

If only four shorts go out: **1a, 2a, 4a, 3b**. They cover the four pitches
(structure, scale, ad hoc, Excel interchange) and none depends on narration.

## Recording automation

The e2e harness drives the real bundled app over WebDriver, so a "demo driver"
spec could perform each take at human pace while `screencapture -v` records.
That gives identical takes on every rerun and is worth it if the videos will
be re-recorded per release. Two cautions before going that way:

- the e2e shell is menu-less, so the Window → Scratchwork Window gesture in
  video 1 cannot be driven that way; record that take by hand;
- WebDriver clicks land instantly, so the driver needs deliberate pauses
  between gestures or the footage is unwatchable at any speed.

For a first pass, record by hand. Four videos is not enough to pay for the
driver.

## Tutorial changes these videos want

None are required, but two would make recording easier:

- a **demo variant of the tour Start workbook** with the paste block already
  in the clipboard instructions removed from the walkthrough card, so the
  card does not fill a third of the frame with text. Simplest is to collapse
  or hide the walkthrough card before recording rather than change the file;
- a **`demo-data` tutorial entry** in the Data Library so video 2 does not
  start with a Finder dialog. Not blocking; the Open file… dialog is one second.
