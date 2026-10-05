# SPDX-License-Identifier: Apache-2.0
"""Independent frozen-profile identity encoder; no Rust code or generated hashes imported."""

import hashlib
import json
import struct
import sys
import unicodedata

PROFILE = "neutral.project-identity/1"
ENVELOPE = "neutral-nht-v1"
DOMAINS = {
    "captured": "neutral/project-captured/v1",
    "logical": "neutral/project-logical/v1",
    "derivation": "neutral/project-derivation/v1",
    "artifact": "neutral/project-artifact/v1",
}
CAPTURE_TAGS = (
    "total-source-bytes", "source-bytes-per-unit", "source-units", "source-id-bytes",
    "module-id-bytes", "vocabulary-units", "vocabulary-bytes-per-unit",
    "total-vocabulary-bytes", "imports-per-module", "import-edges", "scc-units",
    "declarations", "diagnostics", "output-bytes",
)
PROJECT_TAGS = ("modules", "declarations", "import-edges", "nodes", "text-bytes", "artifact-bytes")
EDGE_ORDER = {"type": 0, "reference-type": 1, "value": 2, "reference": 3}
MAX_BYTES = 67_108_864
MAX_NODES = 1_000_000
MAX_DEPTH = 64


class Rejected(Exception):
    """Fail-closed reference classification, never a partial accepted transcript."""


def require(condition, classification="InvalidInput"):
    """Reject a contract violation using the shared external classification names."""
    if not condition:
        raise Rejected(classification)


def ordered(values):
    """Require strict byte/tuple order without silently sorting canonical input."""
    values = list(values)
    require(all(left < right for left, right in zip(values, values[1:])))


def utf8(value):
    """Encode exact Unicode text without normalization or case folding."""
    return value.encode("utf-8")


class Encoder:
    """Independent recursive framing with explicit work accounting."""

    def __init__(self, limits, cancelled=False):
        """Intersect independent limits before constructing any output frames."""
        self.bytes = min(limits["bytes"], MAX_BYTES)
        self.nodes = min(limits["nodes"], MAX_NODES)
        self.count = 0
        self.keys = 0
        require(self.bytes > 0 and self.nodes > 0, "Limit")
        require(not cancelled, "Cancelled")

    def items(self, values):
        """Bound input collection traversal separately from retained frame count."""
        require(len(values) <= self.nodes, "Limit")

    def text(self, value):
        """Bound cumulative comparison-key bytes before sorting or ordering checks."""
        self.keys += len(utf8(value))
        require(self.keys <= self.bytes, "Limit")

    def frame(self, tag, payload=b""):
        """Construct exact u16/tag/u64/payload framing, counting every nested frame."""
        self.count += 1
        require(self.count <= self.nodes, "Limit")
        label = tag.encode("ascii")
        require(len(label) <= 65535, "Limit")
        result = struct.pack(">H", len(label)) + label + struct.pack(">Q", len(payload)) + payload
        require(len(result) <= self.bytes, "Limit")
        return result

    def text_frame(self, tag, value):
        """Frame exact UTF-8 text without introducing delimiter ambiguity."""
        return self.frame(tag, utf8(value))

    def integer(self, tag, value):
        """Frame a fixed-width unsigned integer independently of host endianness."""
        return self.frame(tag, struct.pack(">Q", value))

    def symbol(self, value):
        """Frame stable profile/module/declaration tuples, never allocation labels."""
        return b"".join(self.text_frame(tag, text) for tag, text in zip(("profile", "module", "declaration"), value))

    def field_types(self, fields):
        """Frame canonical unique name/type field pairs."""
        self.items(fields)
        for name, _ in fields:
            self.text(name)
        ordered(utf8(name) for name, _ in fields)
        return b"".join(self.frame("field", self.text_frame("name", name) + self.type(ty)) for name, ty in fields)

    def type(self, ty, depth=0):
        """Encode every core, nominal, vocabulary, and container type distinctly."""
        require(depth <= MAX_DEPTH, "Limit")
        kind = ty["kind"]
        if kind in ("num", "string", "bool", "url", "path"):
            return self.frame(kind)
        if kind in ("List", "Ref", "nullable"):
            return self.frame(kind, self.type(ty["inner"], depth + 1))
        if kind == "nominal":
            return self.frame(kind, self.symbol(ty["symbol"]))
        require(kind == "vocabulary")
        return self.frame(kind, b"".join(self.text_frame(tag, ty[tag]) for tag in ("identity", "version", "name")))

    def fields(self, fields, depth):
        """Encode canonical record/default fields while preserving nested value order."""
        self.items(fields)
        for name, _ in fields:
            self.text(name)
        ordered(utf8(name) for name, _ in fields)
        return b"".join(self.frame("field", self.text_frame("name", name) + self.value(value, depth)) for name, value in fields)

    def value(self, value, depth=0):
        """Encode normalized typed values; references remain stable identity-only edges."""
        require(depth <= MAX_DEPTH, "Limit")
        kind = value["kind"]
        if kind in ("absent", "null"):
            return self.frame(kind)
        if kind == "bool":
            return self.frame(kind, bytes([int(value["value"])]))
        if kind in ("string", "url", "path"):
            return self.text_frame(kind, value["value"])
        if kind == "num":
            return self.frame(kind, self.frame("negative", bytes([int(value["negative"])]))
                              + self.text_frame("coefficient", value["coefficient"])
                              + self.frame("scale", struct.pack(">q", value["scale"])))
        if kind == "Ref":
            return self.frame(kind, self.symbol(value["value"]))
        if kind == "List":
            self.items(value["value"])
            return self.frame(kind, b"".join(self.value(item, depth + 1) for item in value["value"]))
        require(kind == "record")
        return self.frame(kind, self.fields(value["value"], depth + 1))

    def captured(self, capture):
        """Encode the complete exact source/vocabulary closure, excluding controls."""
        sources, vocabularies = capture["sources"], capture["vocabularies"]
        require(capture["profile"] and sources)
        self.items(sources)
        self.items(vocabularies)
        for source in sources:
            self.text(source["module"])
            self.text(source["source_id"])
            require(not any(unicodedata.category(char) == "Cc" for char in source["source_id"]))
        ordered(sorted(utf8(source["source_id"]) for source in sources))
        for vocabulary in vocabularies:
            self.text(vocabulary["identity"])
        ordered(utf8(source["module"]) for source in sources)
        ordered(utf8(vocabulary["identity"]) for vocabulary in vocabularies)
        source_frames = []
        for source in sources:
            require(source["module"] and source["source_id"])
            source_frames.append(self.frame("source", self.text_frame("module", source["module"])
                                + self.text_frame("source-id", source["source_id"])
                                + self.frame("digest", bytes.fromhex(source["digest"]))
                                + self.integer("byte-length", source["byte_len"])))
        vocabulary_frames = []
        for vocabulary in vocabularies:
            self.items(vocabulary["features"])
            for feature in vocabulary["features"]:
                self.text(feature)
            ordered(utf8(feature) for feature in vocabulary["features"])
            header = b""
            for tag, field in (("identity", "identity"), ("version", "version"), ("encoding-version", "encoding_version"), ("schema-version", "schema_version")):
                require(vocabulary[field])
                header += self.text_frame(tag, vocabulary[field])
            vocabulary_frames.append(self.frame("vocabulary", header
                                      + self.frame("digest", bytes.fromhex(vocabulary["digest"]))
                                      + self.integer("byte-length", vocabulary["byte_len"])
                                      + self.frame("features", b"".join(self.text_frame("feature", feature) for feature in vocabulary["features"]))))
        return self.text_frame("profile", capture["profile"]) + self.frame("sources", b"".join(source_frames)) + self.frame("vocabularies", b"".join(vocabulary_frames))

    def logical(self, project):
        """Encode complete canonical typed meaning, including private and disconnected members."""
        modules, declarations, records = project["modules"], project["declarations"], project["records"]
        require(project["schema"] == "neutral.project-ir/1" and modules)
        for collection in (modules, declarations, records):
            self.items(collection)
        imports = []
        for module in modules:
            self.text(module["name"])
            imports.extend(module["imports"])
            self.items(imports)
            for target in module["imports"]:
                self.text(target)
        for declaration in declarations:
            for text in declaration["symbol"]:
                self.text(text)
        for record in records:
            for tag in ("identity", "version", "name"):
                self.text(record[tag])
        ordered((module["profile"], module["name"]) for module in modules)
        ordered(tuple(declaration["symbol"]) for declaration in declarations)
        ordered(tuple(record[tag] for tag in ("identity", "version", "name")) for record in records)
        module_frames = []
        for module in modules:
            require(module["profile"] == "1.0")
            self.items(module["imports"])
            ordered(utf8(target) for target in module["imports"])
            module_frames.append(self.frame("module", self.text_frame("name", module["name"])
                                 + self.frame("imports", b"".join(self.text_frame("target", target) for target in module["imports"]))))
        declaration_frames = []
        for declaration in declarations:
            signature = declaration["signature"]
            body = self.type(signature["type"]) if signature["kind"] == "binding" else self.field_types(signature["fields"])
            declaration_frames.append(self.frame("declaration", self.symbol(declaration["symbol"])
                                      + self.frame("public", bytes([int(declaration["public"])]))
                                      + self.frame("signature", self.frame(signature["kind"], body))
                                      + self.frame("value", self.value(declaration["value"]))
                                      + self.frame("defaults", self.fields(declaration["defaults"], 0))))
        record_frames = [self.frame("record", b"".join(self.text_frame(tag, record[tag]) for tag in ("identity", "version", "name"))
                         + self.frame("public", bytes([int(record["public"])]))
                         + self.frame("fields", self.field_types(record["fields"]))) for record in records]
        catalogues = project["catalogues"]
        self.items(catalogues)
        for catalogue in catalogues:
            self.text(catalogue["identity"])
            self.text(catalogue["version"])
        ordered((catalogue["identity"], catalogue["version"]) for catalogue in catalogues)
        catalogue_frames = []
        for catalogue in catalogues:
            self.items(catalogue["public_types"])
            for name in catalogue["public_types"]:
                self.text(name)
            ordered(utf8(name) for name in catalogue["public_types"])
            catalogue_frames.append(self.frame("vocabulary", self.text_frame("identity", catalogue["identity"])
                                    + self.text_frame("version", catalogue["version"])
                                    + self.frame("public-types", b"".join(self.text_frame("name", name) for name in catalogue["public_types"]))))
        edges = project["edges"]
        self.items(edges)
        for edge in edges:
            for symbol in (edge["from"], edge["to"]):
                for text in symbol:
                    self.text(text)
        ordered((tuple(edge["from"]), EDGE_ORDER[edge["kind"]], tuple(edge["to"])) for edge in edges)
        edge_frames = [self.frame("edge", self.frame("from", self.symbol(edge["from"])) + self.frame("to", self.symbol(edge["to"]))
                       + self.text_frame("kind", edge["kind"])) for edge in edges]
        return (self.text_frame("schema", project["schema"]) + self.text_frame("profile", "1.0")
                + self.frame("modules", b"".join(module_frames)) + self.frame("declarations", b"".join(declaration_frames))
                + self.frame("vocabulary-records", b"".join(record_frames)) + self.frame("vocabulary-catalogues", b"".join(catalogue_frames))
                + self.frame("public-edges", b"".join(edge_frames)))

    def derivation(self, request, captured, logical):
        """Encode exact upstream hashes and explicit producer/acceptance context."""
        require(request["producer"] and request["producer_version"])
        for value in request["capture_limits"] + request["project_limits"]:
            require(value > 0)
        return (self.frame("logical", logical) + self.frame("captured", captured)
                + self.text_frame("producer", request["producer"]) + self.text_frame("producer-version", request["producer_version"])
                + self.frame("capture-limits", b"".join(self.integer(tag, value) for tag, value in zip(CAPTURE_TAGS, request["capture_limits"])))
                + self.frame("project-limits", b"".join(self.integer(tag, value) for tag, value in zip(PROJECT_TAGS, request["project_limits"]))))

    def artifact(self, artifact, derivation):
        """Encode distinct complete/view artifacts, normalized roots, and explicit options."""
        self.text(artifact["format"])
        self.items(artifact["roots"])
        self.items(artifact["options"])
        for symbol in artifact["roots"]:
            for text in symbol:
                self.text(text)
        for name, _ in artifact["options"]:
            self.text(name)
        require(artifact["format"] and artifact["format"].isascii()
                and not any(unicodedata.category(char) == "Cc" for char in artifact["format"]))
        require(artifact["kind"] in ("project", "view"))
        require(artifact["kind"] != "project" or not artifact["roots"])
        roots = sorted(tuple(symbol) for symbol in artifact["roots"])
        ordered(roots)
        ordered(utf8(name) for name, _ in artifact["options"])
        require(all(name for name, _ in artifact["options"]))
        return (self.frame("derivation", derivation) + self.text_frame("kind", artifact["kind"]) + self.text_frame("format", artifact["format"])
                + self.frame("roots", b"".join(self.frame("root", self.symbol(symbol)) for symbol in roots))
                + self.frame("options", b"".join(self.frame("option", self.text_frame("name", name) + self.text_frame("value", value)) for name, value in artifact["options"])))


def encode(request, layer, body):
    """Publish only a complete profile/domain/envelope transcript with hashlib SHA-256."""
    try:
        encoder = Encoder(request["limits"], request.get("cancelled", False))
        payload = encoder.text_frame("identity-profile", PROFILE) + body(encoder)
        transcript = encoder.frame(ENVELOPE, encoder.frame(DOMAINS[layer], payload))
        return {"transcript_hex": transcript.hex(), "sha256": hashlib.sha256(transcript).hexdigest(), "transcript_bytes": len(transcript), "frames": encoder.count}
    except Rejected as error:
        return {"error": str(error)}


def inspect(request):
    """Independently compute each layer; upstream failures prevent downstream identities."""
    operation = request.get("operation")
    if operation:
        upstream = request.get("upstream", {})
        bodies = {
            "captured": lambda encoder: encoder.captured(request["capture"]),
            "logical": lambda encoder: encoder.logical(request["logical"]),
            "derivation": lambda encoder: encoder.derivation(request, bytes.fromhex(upstream["captured"]), bytes.fromhex(upstream["logical"])),
            "artifact": lambda encoder: encoder.artifact(request["artifact"], bytes.fromhex(upstream["derivation"])),
        }
        return encode(request, operation, bodies[operation])
    output = {"captured": encode(request, "captured", lambda encoder: encoder.captured(request["capture"])),
              "logical": encode(request, "logical", lambda encoder: encoder.logical(request["logical"]))}
    if "error" not in output["captured"] and "error" not in output["logical"]:
        captured = bytes.fromhex(output["captured"]["sha256"])
        logical = bytes.fromhex(output["logical"]["sha256"])
        output["derivation"] = encode(request, "derivation", lambda encoder: encoder.derivation(request, captured, logical))
        if "error" not in output["derivation"]:
            digest = bytes.fromhex(output["derivation"]["sha256"])
            for artifact in request["artifacts"]:
                output[artifact["layer"]] = encode(request, "artifact", lambda encoder: encoder.artifact(artifact, digest))
    return output


def main():
    """Read one bounded JSON request and emit indented comparison results, never bless vectors."""
    require(sys.version_info >= (3, 8))
    raw = sys.stdin.buffer.read(MAX_BYTES + 1)
    require(len(raw) <= MAX_BYTES, "Limit")
    print(json.dumps(inspect(json.loads(raw)), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
