# SPDX-License-Identifier: Apache-2.0
"""Independent composition /2 design oracle; consumes canonical resolved facts only."""

import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys

_spec = importlib.util.spec_from_file_location(
    "identity_v1_reference", Path(__file__).parent.parent / "project_identity" / "reference.py"
)
base = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(base)

PROFILE = "neutral.project-identity/2"
SCHEMA = "neutral.project-ir/2"
FEATURES = ["tagged-variants-v1", "vocabulary-composition-v2"]
COMPOSITION_TAGS = (
    "bundles", "captured-bytes", "dependencies-per-bundle", "dependency-edges",
    "dependency-depth", "total-types", "total-fields", "alternatives-per-type",
    "total-alternatives", "choices-per-field", "total-choices", "type-depth",
    "value-depth", "value-nodes", "work",
)
SCALARS = ("num", "string", "bool", "url", "path")


def shape(value, keys):
    """Reject unknown or missing members in a canonical fact node."""
    base.require(isinstance(value, dict) and set(value) == set(keys))


def number_key(value):
    """Order exact normalized decimals without allocating exponent-sized padding."""
    coefficient, scale = value["coefficient"], value["scale"]
    if coefficient == "0":
        return (1, 0, "")
    exponent = len(coefficient) + scale
    if value["negative"]:
        # Complemented digits reverse magnitude; ':' sorts after all complemented
        # digits, so a longer equal prefix has greater magnitude before reversal.
        return (0, -exponent, "".join(str(9 - int(c)) for c in coefficient) + ":")
    return (2, exponent, coefficient)


def scalar_key(value):
    """Supply the frozen same-type mathematical/Boolean/UTF-8 choice ordering."""
    return number_key(value) if value["kind"] == "num" else value["value"] if value["kind"] == "bool" else base.utf8(value["value"])


class Encoder(base.Encoder):
    """Extend independent /1 primitives without mutating its profile or grammar."""

    def symbol(self, value):
        """Require the complete stable tuple rather than truncating malformed input."""
        base.require(len(value) == 3 and value[0] == "1.0" and all(value))
        return super().symbol(value)

    def type(self, ty, depth=0):
        """Reject unsupported wrappers and enforce invariant nominal references."""
        kind = ty["kind"]
        keys = ["kind"] if kind in SCALARS else ["kind", "inner"] if kind in ("List", "Ref", "nullable") else ["kind", "symbol"] if kind == "nominal" else ["kind", "identity", "version", "name"]
        shape(ty, keys)
        if kind == "nullable":
            base.require(ty["inner"]["kind"] != "nullable")
        if kind == "Ref":
            base.require(ty["inner"]["kind"] in ("nominal", "vocabulary"))
        return super().type(ty, depth)

    def value(self, value, depth=0):
        """Frame variants/optional absence while forbidding absence in nested values."""
        base.require(depth <= base.MAX_DEPTH, "Limit")
        kind = value["kind"]
        keys = ["kind"] if kind in ("absent", "omitted", "null") else ["kind", "negative", "coefficient", "scale"] if kind == "num" else ["kind", "tag", "payload"] if kind == "variant" else ["kind", "value"]
        shape(value, keys)
        if kind == "omitted":
            raise base.Rejected("InvalidInput")
        if kind == "variant":
            base.require(value["tag"] and value["payload"]["kind"] not in ("absent", "omitted"))
            return self.frame(kind, self.text_frame("tag", value["tag"]) + self.frame("payload", self.value(value["payload"], depth + 1)))
        if kind == "num":
            c, scale, negative = value["coefficient"], value["scale"], value["negative"]
            base.require(isinstance(negative, bool) and type(scale) is int and -(2**63) <= scale < 2**63)
            base.require(c and c.isascii() and c.isdecimal())
            base.require((c == "0" and scale == 0 and not negative) or (c != "0" and c[0] != "0" and c[-1] != "0"))
        if kind == "bool":
            base.require(isinstance(value["value"], bool))
        if kind in ("List", "record"):
            items = value["value"] if kind == "List" else [v for _, v in value["value"]]
            base.require(all(v["kind"] != "absent" and (kind == "record" or v["kind"] != "omitted") for v in items))
        return super().value(value, depth)

    def fields(self, fields, depth):
        """Permit optional absence only at an explicit record-field occurrence."""
        base.require(depth <= base.MAX_DEPTH, "Limit")
        self.items(fields)
        for name, _ in fields:
            self.text(name)
        base.ordered(base.utf8(name) for name, _ in fields)
        result = b""
        for name, value in fields:
            if value["kind"] == "omitted":
                shape(value, ("kind",))
                payload = self.frame("omitted")
            else:
                base.require(value["kind"] != "absent")
                payload = self.value(value, depth)
            result += self.frame("field", self.text_frame("name", name) + payload)
        return result

    def restrictions(self, value):
        """Emit every restriction slot explicitly; canonical choices are not sorted here."""
        shape(value, ("choices", "minimum", "maximum", "min_length", "max_length"))
        choices = value["choices"]
        self.items(choices)
        encoded = [self.value(v) for v in choices]
        if choices:
            base.require(choices[0]["kind"] in SCALARS and all(v["kind"] == choices[0]["kind"] for v in choices))
            base.ordered(scalar_key(v) for v in choices)
        result = self.frame("choices", b"".join(encoded))
        for name in ("minimum", "maximum", "min_length", "max_length"):
            bound = value[name]
            if bound is None:
                payload = self.frame("absent")
            elif name in ("minimum", "maximum"):
                base.require(bound["kind"] == "num")
                payload = self.value(bound)
            else:
                base.require(type(bound) is int and 0 <= bound < 2**64)
                payload = self.integer("length", bound)
            result += self.frame(name.replace("_", "-"), payload)
        return self.frame("restrictions", result)

    def contract_fields(self, fields):
        """Encode complete field contracts including unused defaults and presence policy."""
        self.items(fields)
        for field in fields:
            shape(field, ("name", "type", "presence", "restrictions", "default"))
            self.text(field["name"])
        base.ordered(base.utf8(field["name"]) for field in fields)
        result = b""
        for field in fields:
            base.require(field["presence"] in ("required", "optional", "defaulted"))
            base.require((field["presence"] == "defaulted") == (field["default"] is not None))
            if field["default"] is not None:
                base.require(field["default"]["kind"] not in ("absent", "omitted", "Ref"))
            default = self.frame("absent") if field["default"] is None else self.value(field["default"])
            result += self.frame("field", self.text_frame("name", field["name"]) + self.type(field["type"])
                                 + self.text_frame("presence", field["presence"]) + self.restrictions(field["restrictions"])
                                 + self.frame("default", default))
        return result

    def signature(self, signature):
        """Use one nominal definition model for source and vocabulary records/variants."""
        kind = signature["kind"]
        if kind == "binding":
            shape(signature, ("kind", "type"))
            return self.frame(kind, self.type(signature["type"]))
        if kind == "record":
            shape(signature, ("kind", "fields"))
            return self.frame(kind, self.contract_fields(signature["fields"]))
        base.require(kind == "variant")
        shape(signature, ("kind", "alternatives"))
        alternatives = signature["alternatives"]
        self.items(alternatives)
        base.require(alternatives)
        for item in alternatives:
            shape(item, ("tag", "type"))
            self.text(item["tag"])
            base.require(item["tag"])
        base.ordered(base.utf8(item["tag"]) for item in alternatives)
        return self.frame(kind, b"".join(self.frame("alternative", self.text_frame("tag", item["tag"]) + self.type(item["type"])) for item in alternatives))

    def captured(self, capture):
        """Bind exact /2 capability selection in addition to the old captured closure."""
        base.require(capture["required_features"] == FEATURES)
        return super().captured(capture) + self.frame("features", b"".join(self.text_frame("feature", f) for f in FEATURES))

    def logical(self, project):
        """Encode all complete /2 meaning without occurrence evidence, roots or aliases."""
        shape(project, ("schema", "profile", "modules", "declarations", "definitions", "catalogues", "dependencies", "edges"))
        base.require(project["schema"] == SCHEMA and project["profile"] == "1.0" and project["modules"])
        for key in ("modules", "declarations", "definitions", "catalogues", "dependencies", "edges"):
            self.items(project[key])
        modules = []
        for m in project["modules"]:
            shape(m, ("name", "imports"))
            self.text(m["name"])
            self.items(m["imports"])
            for target in m["imports"]:
                self.text(target)
            base.ordered(base.utf8(t) for t in m["imports"])
            modules.append(self.frame("module", self.text_frame("name", m["name"]) + self.frame("imports", b"".join(self.text_frame("target", t) for t in m["imports"]))))
        base.ordered(base.utf8(m["name"]) for m in project["modules"])
        declarations = []
        for d in project["declarations"]:
            shape(d, ("symbol", "public", "signature", "value"))
            base.require(isinstance(d["public"], bool))
            base.require((d["signature"]["kind"] != "binding") == (d["value"]["kind"] == "absent"))
            for text in d["symbol"]:
                self.text(text)
            declarations.append(self.frame("declaration", self.symbol(d["symbol"]) + self.frame("public", bytes([int(d["public"])]))
                                           + self.frame("signature", self.signature(d["signature"])) + self.frame("value", self.value(d["value"]))))
        base.ordered(tuple(d["symbol"]) for d in project["declarations"])
        definitions = []
        for d in project["definitions"]:
            shape(d, ("identity", "version", "name", "public", "body"))
            base.require(d["body"]["kind"] in ("record", "variant") and isinstance(d["public"], bool))
            for key in ("identity", "version", "name"):
                self.text(d[key])
            definitions.append(self.frame("definition", b"".join(self.text_frame(k, d[k]) for k in ("identity", "version", "name"))
                                          + self.frame("public", bytes([int(d["public"])])) + self.frame("body", self.signature(d["body"]))))
        base.ordered(tuple(d[k] for k in ("identity", "version", "name")) for d in project["definitions"])
        catalogues = []
        for c in project["catalogues"]:
            shape(c, ("identity", "version", "schema_version", "features", "public_types"))
            base.require((c["schema_version"], c["features"]) in (("1.0", []), ("2.0", ["vocabulary-composition-v2"])))
            for key in ("features", "public_types"):
                self.items(c[key])
                for text in c[key]:
                    self.text(text)
                base.ordered(base.utf8(t) for t in c[key])
            catalogues.append(self.frame("vocabulary", self.text_frame("identity", c["identity"]) + self.text_frame("version", c["version"])
                                         + self.text_frame("schema-version", c["schema_version"])
                                         + self.frame("features", b"".join(self.text_frame("feature", t) for t in c["features"]))
                                         + self.frame("public-types", b"".join(self.text_frame("name", t) for t in c["public_types"]))))
        dependencies = []
        for d in project["dependencies"]:
            shape(d, ("identity", "version", "dependencies"))
            self.items(d["dependencies"])
            base.ordered(tuple(v) for v in d["dependencies"])
            base.require(all(len(v) == 2 and all(v) for v in d["dependencies"]))
            dependencies.append(self.frame("vocabulary", self.text_frame("identity", d["identity"]) + self.text_frame("version", d["version"])
                                           + self.frame("dependencies", b"".join(self.frame("dependency", self.text_frame("identity", v[0]) + self.text_frame("version", v[1])) for v in d["dependencies"]))))
        for key in ("catalogues", "dependencies"):
            for d in project[key]:
                self.text(d["identity"])
                self.text(d["version"])
            base.ordered((d["identity"], d["version"]) for d in project[key])
        base.require([(d["identity"], d["version"]) for d in project["dependencies"]] == [(c["identity"], c["version"]) for c in project["catalogues"]])
        edges = []
        for e in project["edges"]:
            shape(e, ("from", "to", "kind"))
            base.require(e["kind"] in base.EDGE_ORDER)
            edges.append(self.frame("edge", self.frame("from", self.symbol(e["from"])) + self.frame("to", self.symbol(e["to"])) + self.text_frame("kind", e["kind"])))
        base.ordered((tuple(e["from"]), base.EDGE_ORDER[e["kind"]], tuple(e["to"])) for e in project["edges"])
        return (self.text_frame("schema", SCHEMA) + self.text_frame("profile", "1.0")
                + self.frame("modules", b"".join(modules)) + self.frame("declarations", b"".join(declarations))
                + self.frame("vocabulary-types", b"".join(definitions)) + self.frame("vocabulary-catalogues", b"".join(catalogues))
                + self.frame("vocabulary-dependencies", b"".join(dependencies)) + self.frame("public-edges", b"".join(edges)))

    def derivation(self, request, captured, logical):
        """Bind all explicit composition budgets in addition to existing derivation inputs."""
        base.require(len(request["capture_limits"]) == len(base.CAPTURE_TAGS) and len(request["project_limits"]) == len(base.PROJECT_TAGS))
        limits = request["composition_limits"]
        base.require(len(limits) == len(COMPOSITION_TAGS) and all(type(v) is int and 0 < v < 2**64 for v in limits))
        return super().derivation(request, captured, logical) + self.frame("composition-limits", b"".join(self.integer(tag, n) for tag, n in zip(COMPOSITION_TAGS, limits)))


def encode(request, layer, body):
    """Return complete literal bytes/hash or a bounded classification; never bless vectors."""
    try:
        encoder = Encoder(request["limits"], request.get("cancelled", False))
        transcript = encoder.frame(base.ENVELOPE, encoder.frame("neutral/project-" + layer + "/v2", encoder.text_frame("identity-profile", PROFILE) + body(encoder)))
        return {"transcript_hex": transcript.hex(), "sha256": hashlib.sha256(transcript).hexdigest(), "transcript_bytes": len(transcript), "frames": encoder.count}
    except base.Rejected as error:
        return {"error": str(error)}


def inspect(request):
    """Compute identity partitions independently, allowing direct rejection-layer requests."""
    operation = request.get("operation")
    if operation == "captured":
        return encode(request, "captured", lambda e: e.captured(request["capture"]))
    if operation == "logical":
        return encode(request, "logical", lambda e: e.logical(request["logical"]))
    if operation == "derivation":
        return encode(request, "derivation", lambda e: e.derivation(request, bytes.fromhex(request["upstream"]["captured"]), bytes.fromhex(request["upstream"]["logical"])))
    if operation == "artifact":
        return encode(request, "artifact", lambda e: e.artifact(request["selected_artifact"], bytes.fromhex(request["upstream"]["derivation"])))
    result = {}
    result["captured"] = encode(request, "captured", lambda e: e.captured(request["capture"]))
    result["logical"] = encode(request, "logical", lambda e: e.logical(request["logical"]))
    if any("error" in v for v in result.values()):
        return result
    result["derivation"] = encode(request, "derivation", lambda e: e.derivation(request, bytes.fromhex(result["captured"]["sha256"]), bytes.fromhex(result["logical"]["sha256"])))
    if "error" in result["derivation"]:
        return result
    for artifact in request["artifacts"]:
        result[artifact["layer"]] = encode(request, "artifact", lambda e: e.artifact(artifact, bytes.fromhex(result["derivation"]["sha256"])))
    return result


if __name__ == "__main__":
    print(json.dumps(inspect(json.load(sys.stdin)), indent=2))
