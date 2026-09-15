Create a retail 4-5-4 (NRF) fiscal calendar: fiscal year starting in
February, the year ending on the Saturday nearest January 31, weekends
Saturday and Sunday, and a holiday on 2026-01-01. Set it as the document's
default calendar.

Build a small frame named "Spans" with a Start and an End date column and
these three rows: (2026-01-01, 2026-01-07), (2026-01-07, 2026-01-01), and
(2026-01-03, 2026-01-04). Add a calculated column `Days` using
`networkdays(\`Start\`, \`End\`)`, then a calculated column `Week` using
`fiscal_week(\`Start\`)`. Save the workbook.
