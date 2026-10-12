#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Independently checks candidate literal arithmetic, NHT/CBOR bytes and corpus integrity.
"""Review tool only: no Neutral parser, production imports, evaluator or backend effects."""

import hashlib
import itertools
import json
from fractions import Fraction
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON member: " + key)
        result[key] = value
    return result


def number(value):
    """Normalize a finite decimal without floating-point conversion."""
    ratio = Fraction(value)
    negative = ratio < 0
    numerator, denominator = abs(ratio.numerator), ratio.denominator
    scale = 0
    while denominator != 1:
        require(denominator % 2 == 0 or denominator % 5 == 0, "not finite decimal")
        if denominator % 2 == 0:
            numerator *= 5
            denominator //= 2
        else:
            numerator *= 2
            denominator //= 5
        scale -= 1
    if not numerator:
        return {"negative": False, "coefficient": "0", "scale": 0}
    while numerator % 10 == 0:
        numerator //= 10
        scale += 1
    return {"negative": negative, "coefficient": str(numerator), "scale": scale}


def divide(left, right):
    quotient = Fraction(left) / Fraction(right)
    denominator = quotient.denominator
    for factor in (2, 5):
        while denominator % factor == 0:
            denominator //= factor
    if denominator == 1:
        return number(quotient)
    magnitude = abs(quotient)
    exponent = len(str(magnitude.numerator)) - len(str(magnitude.denominator))
    if magnitude < Fraction(10) ** exponent:
        exponent -= 1
    scale = exponent - 33
    scaled = magnitude / Fraction(10) ** scale
    coefficient, remainder = divmod(scaled.numerator, scaled.denominator)
    comparison = 2 * remainder - scaled.denominator
    if comparison > 0 or (comparison == 0 and coefficient % 2):
        coefficient += 1
    rounded = Fraction(coefficient) * Fraction(10) ** scale
    return number(-rounded if quotient < 0 else rounded)


def nht(frame):
    tag = frame["tag"].encode("ascii")
    require(0 < len(tag) <= 65535, "invalid frame tag")
    if "children" in frame:
        require(set(frame) == {"tag", "children"}, "invalid container frame")
        payload = b"".join(nht(child) for child in frame["children"])
    else:
        require(set(frame) == {"tag", "payload"}, "invalid scalar frame")
        scalar = frame["payload"]
        require(len(scalar) == 1, "invalid scalar payload")
        kind, value = next(iter(scalar.items()))
        if kind == "text":
            payload = value.encode("utf-8")
        elif kind == "hex":
            payload = bytes.fromhex(value)
        elif kind in ("uint", "int"):
            payload = value.to_bytes(8, "big", signed=kind == "int")
        else:
            raise ValueError("unknown scalar kind")
    return len(tag).to_bytes(2, "big") + tag + len(payload).to_bytes(8, "big") + payload


def cbor(value):
    def header(major, size):
        if size < 24:
            return bytes([(major << 5) | size])
        for width, additional in ((1, 24), (2, 25), (4, 26), (8, 27)):
            if size < 1 << (8 * width):
                return bytes([(major << 5) | additional]) + size.to_bytes(width, "big")
        raise ValueError("CBOR size overflow")

    if value is None:
        return b"\xf6"
    if isinstance(value, bool):
        return b"\xf5" if value else b"\xf4"
    if isinstance(value, int):
        return header(0, value) if value >= 0 else header(1, -1 - value)
    if isinstance(value, str):
        data = value.encode("utf-8")
        return header(3, len(data)) + data
    if isinstance(value, list):
        return header(4, len(value)) + b"".join(cbor(item) for item in value)
    if isinstance(value, dict) and set(value) == {"bytes"}:
        data = bytes.fromhex(value["bytes"])
        return header(2, len(data)) + data
    raise ValueError("not restricted CBOR")


def self_check():
    require(divide("1", "8") == number("0.125"), "terminating division")
    require(divide("1", "3") == number("0." + "3" * 34), "rounded division")
    require(divide("-1", "6") == number("-0.1" + "6" * 32 + "7"), "signed rounding")
    require(number("-0.00") == number("0"), "canonical zero")
    try:
        divide("1", "0")
    except ZeroDivisionError:
        pass
    else:
        raise ValueError("division by zero accepted")
    require(cbor([0, 24, -1, True, None, "é"]).hex() == "8600181820f5f662c3a9",
            "CBOR scalar encodings")
    require(nht({"tag": "x", "payload": {"text": "é"}}).hex() ==
            "0001780000000000000002c3a9", "NHT UTF-8 length")
    try:
        require("00" == "01", "deliberately corrupted oracle")
    except ValueError:
        pass
    else:
        raise ValueError("oracle mismatch not rejected")


def main():
    self_check()
    root = Path(__file__).resolve().parents[4]
    fixture = root / "portable/specs/fixtures/expressions"
    cases = json.loads((fixture / "cases.json").read_text(), object_pairs_hook=unique_object)
    outcomes = json.loads((Path(__file__).parent / "results.json").read_text(),
                          object_pairs_hook=unique_object)
    vectors = json.loads((fixture / "vectors.json").read_text(), object_pairs_hook=unique_object)
    complete = json.loads((fixture / "complete-vectors.json").read_text(),
                          object_pairs_hook=unique_object)
    authoring = json.loads((fixture / "authoring-vectors.json").read_text(),
                           object_pairs_hook=unique_object)
    from project_vectors import verify_registered
    verify_registered(root, vectors, complete, authoring)
    ids = [case["id"] for case in cases["cases"]]
    require(len(ids) == len(set(ids)), "duplicate case IDs")
    require(set(ids) == {case["id"] for case in outcomes["cases"]}, "oracle coverage")
    require(len(outcomes["cases"]) == len(ids), "duplicate oracle IDs")
    registered = {item["path"] for item in cases["files"]}
    for item in cases["files"]:
        path = (root / item["path"]).resolve()
        require(path.is_relative_to(root), "path escape")
        require(hashlib.sha256(path.read_bytes()).hexdigest() == item["sha256"],
                "fixture digest mismatch: " + item["path"])
    for case in cases["cases"]:
        sources = [case["source"]] + case.get("other_sources", [])
        if "other_source" in case:
            sources.append(case["other_source"])
        if "vocabulary" in case:
            sources.append(case["vocabulary"])
        for source in sources:
            require(str((fixture / source).relative_to(root)) in registered,
                    "unregistered input: " + source)
    baseline = vectors["baseline"]
    require((root / baseline["source"]).read_text() == baseline["source_utf8"],
            "baseline source bytes")
    require(hashlib.sha256(baseline["source_utf8"].encode()).hexdigest() ==
            baseline["source_sha256"], "baseline source digest")
    for vector in vectors["arithmetic"]:
        result = divide(*vector["operands"])
        require(result == vector["expected"], "arithmetic mismatch: " + vector["id"])
    for vector in vectors["transcripts"]:
        data = nht(vector["frames"])
        require(data.hex() == vector["transcript_hex"], "transcript: " + vector["id"])
        require(hashlib.sha256(data).hexdigest() == vector["sha256"],
                "digest: " + vector["id"])
    for vector in vectors["wire"]:
        data = bytes.fromhex(vector.get("magic_hex", "")) + cbor(vector["tuple"])
        require(data.hex() == vector["encoded_hex"], "wire: " + vector["id"])
        require(hashlib.sha256(data).hexdigest() == vector["sha256"],
                "wire digest: " + vector["id"])
    for vector in vectors["matrices"]:
        values = []
        for indices in itertools.product(*(range(len(axis)) for axis in vector["axes"])):
            items = [axis[index] for axis, index in zip(vector["axes"], indices)]
            if items in vector["exclude"]:
                continue
            values.append({"symbol": vector["name"] + "".join("__" + str(i) for i in indices),
                           "value": "-".join(items)})
        require(values == vector["expected"], "matrix: " + vector["id"])
    print(f"PASS candidate integrity: {len(ids)} cases, {len(cases['files'])} inputs; "
          f"{len(vectors['arithmetic'])} rational results, "
          f"{len(vectors['transcripts'])} NHT transcripts, "
          f"{len(vectors['wire'])} CBOR vectors, {len(vectors['matrices'])} matrices")
    print(f"PASS additional candidates: {len(complete['projects'])} complete projects, "
          f"{len(complete['identity_variations'])} identity variations, "
          f"{len(authoring['valid'])} authoring projections, "
          f"{len(authoring['invalid'])} authoring rejections")
    print("NOT CHECKED: Neutral parsing/evaluation, hostile decoding, or production comparisons")


if __name__ == "__main__":
    main()
