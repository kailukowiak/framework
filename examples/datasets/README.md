# FrameWork dataset examples

Generate the local sample library with:

```sh
cargo run -p framework-core --example generate_sample_documents
```

The command writes `.fw` documents to the git-ignored `.framework-samples/` directory. The desktop Data library discovers files recursively and displays their category.

## Canonical datasets

- **Anscombe's quartet** — Francis Anscombe's 1973 dataset, represented in long form with a small series lookup table. The published values are reproduced by the generator.
- **UC Berkeley admissions 1973** — the aggregated `UCBAdmissions` counts distributed with R, plus a department lookup table. It is commonly used to demonstrate Simpson's paradox.

## Synthetic datasets

- **Commerce join playground** — the built-in compact retail model.
- **Subscription analytics** — deterministic accounts, plans, subscriptions, and payments.
- **Support operations** — deterministic tickets, customers, agents, and SLA policies.
- **Excel import workbook** — a two-sheet `.xlsx` containing three defined
  Excel Tables. The generated FrameWork sample imports Inventory and Suppliers
  from one worksheet and Orders from the other as cached, static values.

Synthetic data uses a fixed pseudo-random seed, so regenerating the library preserves its shape and edge cases. Missing foreign keys are intentional and make left-versus-inner join behavior easy to inspect.

## Sources

- Anscombe, F. J. (1973), [“Graphs in Statistical Analysis”](https://doi.org/10.1080/00031305.1973.10478966), *The American Statistician*, 27(1), 17–21.
- R datasets documentation: [`UCBAdmissions`, “Student Admissions at UC Berkeley”](https://search.r-project.org/R/refmans/datasets/html/UCBAdmissions.html).
