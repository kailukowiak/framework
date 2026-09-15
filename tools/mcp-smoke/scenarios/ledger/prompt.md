You are smoke-testing FrameWork, a reactive spreadsheet successor, through its
MCP tools (server "framework"). Your job is to post a small journal that
balances, and to report honestly how easy or hard it was. The document you are
connected to starts as a demo — ignore or delete its demo objects as you
prefer.

Somebody has handed you six journal lines off a printout. Debits are plain,
credits are in parentheses, and one amount still has its dollar sign on it.
Type them in exactly as written:

| Line | Memo               | Amount     |
| ---- | ------------------ | ---------- |
| 1    | Invoice 4471       | $1,234.56  |
| 2    | Payment on account | (1,000.00) |
| 3    | Settlement         | (234.56)   |
| 4    | Postage            | 0.10       |
| 5    | Stationery         | 0.20       |
| 6    | Petty cash in      | (0.30)     |

Hard requirements:

1. A frame holding those six lines, with the amounts entered as written above.
   Do not pre-convert them to plain signed decimals and do not round them.
2. The amount column must be an exact accounting column carrying two decimal
   places — not a float. Money that has to balance cannot be a float: the
   whole point of this scenario is that the control total below reads exactly
   zero rather than nearly zero.
3. A calculated running balance column on the same frame — each row's balance
   is every amount up to and including that row, in line-number order. It must
   itself be an exact amount, not a float. A running calculation needs the
   frame's row order declared first; the error will tell you so.
4. A control total: a saved sum of the amount column on the frame, which must
   read exactly 0.00. Debits minus credits is zero; a journal that does not
   foot is not posted.

Work through the MCP tools only. When something fails, read the error and
adapt — the errors are written to guide you. When you cannot find a
capability, use the discovery tools before concluding it doesn't exist.

End your run with a report:

- FRICTION LOG: each place you got stuck, what you tried, what worked.
- SEARCHES: what you searched for and whether the results led you to the
  answer.
- VERIFICATION: the control total, and the running balance on each of the six
  lines.
- VERDICT: could a bookkeeper have done this through a UI offering these same
  operations? What single change would most improve the experience?
