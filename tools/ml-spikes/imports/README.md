# Import reference spike

This is development-only Python. It creates tiny synthetic fitted pipelines,
exports them, records schemas and learned mappings, and checks reference-library
parity. It does not accept arbitrary user models or implement FrameWork import.

Run from the repository root with Python 3.12:

```sh
python3.12 -m venv /tmp/framework-ml-imports
/tmp/framework-ml-imports/bin/pip install -r tools/ml-spikes/imports/requirements-lock.txt
/tmp/framework-ml-imports/bin/python tools/ml-spikes/imports/generate.py
```

`requirements.txt` selects the direct reference versions; `requirements-lock.txt`
records the complete tested environment. No Python dependency is added to the
application. XGBoost's wheel may require a platform OpenMP runtime.

The checked-in `fixtures/` directory contains ONNX protobufs, raw XGBoost JSON,
and readable sidecars. All sample rows are generated synthetic data. JSON null
in numeric probes means a missing value and must become NaN at the ONNX/XGBoost
input boundary; string missing values deliberately use an explicit empty-string
sentinel. These are different policies.

The generator asserts ONNX Runtime versus sklearn parity and restored native
XGBoost versus sklearn-wrapper parity. Conversion rejection is recorded as a
distinct result rather than mislabeled a pass; execution/parity failures stop
the run. Inspect `report.json` case statuses. `--output /tmp/other-fixtures`
supports regeneration without changing the checked-in reference fixtures.

See [the evidence report](../../../docs/ml-import-spike.md) for limitations and
the remaining native translator/tree execution acceptance work.
