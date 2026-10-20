#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Validate the current oracle revision and the unchanged historical vector snapshots.
"""A reviewed diagnostic erratum does not amend expression/evaluator semantics."""

import hashlib
import json
from pathlib import Path

from verify import main as verify_historical, require, unique_object


def main():
    verify_historical()
    folder = Path(__file__).parent
    original = (folder / "results.json").read_bytes()
    old = json.loads(original, object_pairs_hook=unique_object)
    new = json.loads((folder / "results-v2.json").read_text(), object_pairs_hook=unique_object)
    require(new["schema"] == "neutral.expression-oracle/2", "unsupported oracle revision")
    require(new["supersedes"]["sha256"] == hashlib.sha256(original).hexdigest(), "historical bytes changed")
    corrections = {"residual-depth-one-over", "residual-nodes-one-over"}
    for case in old["cases"]:
        if case["id"] in corrections:
            require(case["code"] == "NEU-EXP-009", "unexpected prior diagnostic")
            case["code"] = "NEU-EVAL-006"
    require(old["cases"] == new["cases"], "unreviewed result change")
    print("PASS current expression oracle /2: two diagnostic corrections; historical inputs unchanged")


if __name__ == "__main__":
    main()
