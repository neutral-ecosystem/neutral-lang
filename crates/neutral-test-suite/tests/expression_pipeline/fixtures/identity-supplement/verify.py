# SPDX-License-Identifier: Apache-2.0

"""Read-only independent source/identity/result comparison; never invokes Neutral or rewrites literals."""

import hashlib
import json
from pathlib import Path

from framing import divide, number, require
from reference import encoded_project, project_partitions


def verify():
    """Recompute every registered transcript, complete frame and rational result."""
    folder = Path(__file__).parent
    vectors = json.loads((folder / "vectors.json").read_text())
    require(vectors["schema"] == "neutral.headless-identity-supplement/1", "supplement schema")
    baselines = json.loads((folder / "baselines.json").read_text())
    for case in baselines["projects"]:
        wire = case["tuple"]
        source = (folder.parent / case["source"]).read_bytes()
        require(hashlib.sha256(source).hexdigest() == wire[6][0][2]["bytes"], "frozen baseline source")
        require(project_partitions(wire, baselines["profiles"], case["context"]) == case["partitions"], "unchanged baseline partitions")
        require(encoded_project(wire).hex() == case["encoded_hex"], "unchanged baseline wire")
    seen = set()
    for case in vectors["cases"]:
        require(case["id"] not in seen, "duplicate case")
        seen.add(case["id"])
        source = (folder / case["source"]).read_bytes()
        wire = case["tuple"]
        require(len(wire) == 22, "complete tuple arity")
        require(hashlib.sha256(source).hexdigest() == wire[6][0][2]["bytes"], "source checksum")
        require(len(source) == wire[6][0][3] == wire[10][1], "source byte accounting")
        require(project_partitions(wire, vectors["profiles"], case["context"]) == case["partitions"], "partition transcripts/digests")
        require(wire[5]["bytes"] == case["partitions"]["interface"]["sha256"], "interface companion")
        encoded = encoded_project(wire)
        require(encoded.hex() == case["encoded_hex"], "complete wire bytes")
        require(hashlib.sha256(encoded).hexdigest() == case["wire_sha256"], "wire checksum")
        tree = wire[2][2][3][1]
        operator = tree[3][2]
        right = tree[3][4][2][2]
        otherwise = tree[4][2][2]
        require(f"count {operator} {right}".encode() in source, "source operator/operand")
        require(f"else {{ {otherwise} }}".encode() in source, "source alternative")
        left = case["bindings"]["count"]
        if not case["bindings"]["enabled"]:
            expected = number(otherwise)
        elif operator == "/":
            expected = divide(left, right)
        else:
            operations = {"+": lambda: int(left) + int(right), "-": lambda: int(left) - int(right), "*": lambda: int(left) * int(right)}
            expected = number(operations[operator]())
        require(expected == case["expected_number"], "literal rational evaluation")
    print(f"PASS {len(seen)} independent complete identity/result vectors")


if __name__ == "__main__":
    verify()
