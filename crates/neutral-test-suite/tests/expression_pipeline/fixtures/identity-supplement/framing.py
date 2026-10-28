# SPDX-License-Identifier: Apache-2.0

"""Independent literal NHT/CBOR framing and rational decimal arithmetic; no Neutral imports."""

from fractions import Fraction

def require(condition, message):
    if not condition:
        raise ValueError(message)


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


