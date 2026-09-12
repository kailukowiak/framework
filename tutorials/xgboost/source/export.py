#!/usr/bin/env python3
"""Export train and unlabeled scoring rows from scikit-learn 1.7.2 Iris."""
from __future__ import annotations

import csv
import hashlib
import io
from pathlib import Path
from urllib.request import urlopen

URL = "https://raw.githubusercontent.com/scikit-learn/scikit-learn/1.7.2/sklearn/datasets/data/iris.csv"
SHA256 = "f13ffa8fdd56fd8e6c8d16d4081a3fbd3114bcd0aae4256c43205169cd9d1449"
payload = urlopen(URL).read()
if hashlib.sha256(payload).hexdigest() != SHA256:
    raise RuntimeError("checksum mismatch for iris.csv")
rows = list(csv.reader(io.StringIO(payload.decode())))[1:]
held_out = {49, 99, 149}
training = [row for index, row in enumerate(rows) if index not in held_out]
scoring = [row[:4] for index, row in enumerate(rows) if index in held_out]
header = ["Sepal length", "Sepal width", "Petal length", "Petal width", "Species code"]
root = Path(__file__).parent
for name, selected, columns in (("training.tsv", training, header), ("scoring.tsv", scoring, header[:4])):
    with (root / name).open("w", newline="") as output:
        csv.writer(output, delimiter="\t", lineterminator="\n").writerows([columns, *selected])
