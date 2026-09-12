#!/usr/bin/env python3
"""Export the exact teaching columns from scikit-learn 1.7.2's bundled data."""
from __future__ import annotations

import csv
import gzip
import hashlib
import io
from pathlib import Path
from urllib.request import urlopen

TAG = "1.7.2"
BASE = f"https://raw.githubusercontent.com/scikit-learn/scikit-learn/{TAG}/sklearn/datasets/data"
FILES = {
    "diabetes_data_raw.csv.gz": "a3e94cc7cea00f8a84fa5f6345203913a68efa42df18f87ddf9bead721bfd503",
    "diabetes_target.csv.gz": "8e53f65eb811df43c206f3534bb3af0e5fed213bc37ed6ba36310157d6023803",
}


def fetch(name: str) -> bytes:
    payload = urlopen(f"{BASE}/{name}").read()
    if hashlib.sha256(payload).hexdigest() != FILES[name]:
        raise RuntimeError(f"checksum mismatch for {name}")
    return gzip.decompress(payload)


data = list(csv.reader(io.StringIO(fetch("diabetes_data_raw.csv.gz").decode()), delimiter=" ", skipinitialspace=True))
target = [float(row[0]) for row in csv.reader(io.StringIO(fetch("diabetes_target.csv.gz").decode()))]
rows = [[row[2], row[3], f"{answer:g}"] for row, answer in zip(data, target, strict=True)]
root = Path(__file__).parent
for name, selected in (("training.tsv", rows[:-3]), ("scoring.tsv", [row[:2] for row in rows[-3:]])):
    header = ["BMI", "Blood pressure", "Progression"] if name == "training.tsv" else ["BMI", "Blood pressure"]
    with (root / name).open("w", newline="") as output:
        csv.writer(output, delimiter="\t", lineterminator="\n").writerows([header, *selected])
