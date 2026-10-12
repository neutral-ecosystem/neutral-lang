# SPDX-License-Identifier: Apache-2.0
# Builds independent candidate /3 identity partitions from the registered minimal project tuples.
"""Fixture oracle, not an untrusted IR validator; no production Neutral imports."""

import hashlib
import copy

from verify import cbor, nht, require


def container(tag, children=()):
    return {"tag": tag, "children": list(children)}


def scalar(tag, kind, value):
    return {"tag": tag, "payload": {kind: value}}


def text(tag, value):
    return scalar(tag, "text", value)


def symbol(value):
    return [text("profile", value[0][0]), text("module", value[0][1]),
            text("declaration", value[1])]


def type_frame(value):
    tag, *parts = value
    if tag in ("num", "string", "bool", "url", "path"):
        require(not parts, "scalar type arity")
        return container(tag)
    if tag in ("List", "Ref", "nullable"):
        return container(tag, [type_frame(parts[0])])
    if tag == "nominal":
        return container(tag, symbol(parts[0]))
    raise ValueError("type outside minimal project oracle scope")


def value_frame(value):
    tag, *parts = value
    if tag in ("null", "omitted"):
        return container(tag)
    if tag == "num":
        negative, coefficient, scale = parts
        return container(tag, [scalar("negative", "hex", "01" if negative else "00"),
                               text("coefficient", coefficient), scalar("scale", "int", scale)])
    if tag == "bool":
        return scalar(tag, "hex", "01" if parts[0] else "00")
    if tag in ("string", "url", "path"):
        return text(tag, parts[0])
    if tag == "List":
        return container(tag, map(value_frame, parts[0]))
    raise ValueError("value outside minimal project oracle scope")


def expression_frame(node):
    kind, resolved_type, *parts = node
    fields = [text("kind", kind), container("type", [type_frame(resolved_type)])]
    if kind == "value":
        fields.append(container("value", [value_frame(parts[0])]))
    elif kind == "input":
        fields.append(container("symbol", symbol(parts[0])))
    elif kind == "bound":
        fields += [scalar("depth", "uint", parts[0]), scalar("slot", "uint", parts[1])]
    elif kind == "binary":
        fields += [text("operator", parts[0]),
                   container("left", [expression_frame(parts[1])]),
                   container("right", [expression_frame(parts[2])])]
    elif kind == "if":
        fields += [container(tag, [expression_frame(child)])
                   for tag, child in zip(("guard", "then", "else"), parts)]
    else:
        raise ValueError("node outside minimal project oracle scope")
    return container("expression", fields)


def semantic_profiles(profiles):
    return container("semantic-profiles", [text(key, profiles[key])
                     for key in ("expression", "evaluator", "expansion", "decimal")])


def template_frame(template):
    owner, parameters, result_type, body = template
    fields = []
    for name, phase, resolved_type, default in parameters:
        fields.append(container("parameter", [text("name", name), text("phase", phase),
                      container("type", [type_frame(resolved_type)]),
                      container("default", [container("absent") if default is None
                                            else expression_frame(default)])]))
    return container("template", symbol(owner) + [container("parameters", fields),
                     container("result-type", [type_frame(result_type)]),
                     container("body", [expression_frame(body)])])


def transcript(domain, profiles, body):
    return container("neutral-nht-v1", [container(domain, [
        text("identity-profile", profiles["identity"]), *body])])


def digest(frame):
    data = nht(frame)
    return {"transcript_hex": data.hex(), "sha256": hashlib.sha256(data).hexdigest()}


def logical_frame(wire, profiles, interface=False):
    require(not any(wire[index] for index in (3, 4, 8, 11, 12)),
            "oracle handles source-only projects without structural provenance")
    declarations = []
    for owner, public, signature, body in wire[2]:
        if interface and not public:
            continue
        require(signature[0] == "binding", "minimal oracle signature")
        if body[0] == "input":
            payload = container("external-input")
        elif body[0] == "value":
            payload = container("constant", [value_frame(body[1])])
        elif body[0] == "expression":
            payload = container("expression", [expression_frame(body[1])])
        else:
            raise ValueError("unknown binding body")
        declarations.append(container("declaration", symbol(owner) + [
            scalar("public", "hex", "01" if public else "00"),
            container("signature", [container("binding", [type_frame(signature[1])])]),
            container("value", [payload])]))
    modules = [container("module", [text("name", module[0][1]),
               container("imports", [text("target", item[1]) for item in module[1]])])
               for module in wire[1]]
    body = [text("schema", wire[0]), text("profile", "1.0"), container("modules", modules),
            container("declarations", declarations), container("vocabulary-types"),
            container("vocabulary-catalogues"), container("vocabulary-dependencies"),
            container("public-edges"), semantic_profiles(profiles),
            container("required-features", [text("feature", name) for name in wire[16][4]]),
            container("public-templates", map(template_frame, wire[17]))]
    return transcript("neutral/project-" + ("interface" if interface else "logical") + "/v3",
                      profiles, body)


def project_partitions(wire, profiles, context):
    """Rebuild all five partitions from explicit complete facts and derivation controls."""
    logical = digest(logical_frame(wire, profiles))
    interface = digest(logical_frame(wire, profiles, interface=True))
    sources = [container("source", [text("module", module), text("source-id", source_id),
               scalar("digest", "hex", checksum["bytes"]), scalar("byte-length", "uint", size)])
               for module, source_id, checksum, size in wire[6]]
    captured = digest(transcript("neutral/project-captured/v3", profiles, [
        text("profile", "1.0"), container("sources", sources), container("vocabularies"),
        container("features", [text("feature", name) for name in wire[16][4]]),
        semantic_profiles(profiles)]))
    controls = context["controls"]
    derivation_body = [scalar("logical", "hex", logical["sha256"]),
                       scalar("captured", "hex", captured["sha256"]),
                       text("producer", context["producer"]),
                       text("producer-version", context["producer_version"])]
    for key in ("capture-limits", "project-limits", "composition-limits"):
        derivation_body.append(container(key, [scalar(name, "uint", value)
                               for name, value in controls[key]]))
    derivation_body.append(semantic_profiles(profiles))
    for key in ("expression-limits", "expansion-limits"):
        derivation_body.append(container(key, [scalar(name, "uint", value)
                               for name, value in controls[key]]))
    derivation = digest(transcript("neutral/project-derivation/v3", profiles, derivation_body))
    artifact = digest(transcript("neutral/project-artifact/v3", profiles, [
        scalar("derivation", "hex", derivation["sha256"]), text("kind", context["kind"]),
        text("format", context["format"]),
        container("roots", [container("root", symbol(owner)) for owner in context["roots"]]),
        container("options", [container("option", [text("name", name), text("value", value)])
                              for name, value in context["options"]])]))
    return {"captured": captured, "logical": logical, "interface": interface,
            "derivation": derivation, "artifact": artifact}


def encoded_project(wire):
    return bytes.fromhex("4e455550330d0a1a") + cbor(wire)


def edited(value, edits):
    result = copy.deepcopy(value)
    for path, replacement in edits:
        require(bool(path), "edit must address a field")
        target = result
        for part in path[:-1]:
            target = target[part]
        target[path[-1]] = copy.deepcopy(replacement)
    return result


def verify_registered(root, original, complete, authoring):
    from authoring_vectors import project

    profiles = complete["profiles"]
    static = original["wire"][0]["tuple"]
    baseline = {"tuple": static, "context": complete["static_context"]}
    baseline["partitions"] = project_partitions(static, profiles, baseline["context"])
    for name, expected in zip(("captured", "logical", "interface", "derivation", "artifact"),
                              original["transcripts"][:5]):
        require(baseline["partitions"][name]["sha256"] == expected["sha256"],
                "historical candidate baseline: " + name)
    bases = {"static": baseline}
    for key, item in zip(("residual", "template"), complete["projects"]):
        wire = item["tuple"]
        require(len(wire) == 22, "complete envelope length")
        require(project_partitions(wire, profiles, item["context"]) == item["partitions"],
                "complete partition bytes: " + item["id"])
        require(wire[5]["bytes"] == item["partitions"]["interface"]["sha256"],
                "interface companion")
        data = encoded_project(wire)
        require(data.hex() == item["encoded_hex"] and
                hashlib.sha256(data).hexdigest() == item["sha256"], "complete wire bytes")
        source = (root / item["source"]).read_bytes()
        checksum = hashlib.sha256(source).hexdigest()
        require(wire[6][0][2]["bytes"] == checksum and wire[6][0][3] == len(source),
                "complete captured source")

        def location(value):
            require(value[0]["bytes"] == checksum and 0 <= value[1] < value[2] <= len(source),
                    "source occurrence bounds")
            source[:value[1]].decode("utf-8")
            source[value[1]:value[2]].decode("utf-8")

        for _, occurrence in wire[7]:
            location(occurrence)
        for _, _, occurrence, related in wire[21][0]:
            if occurrence is not None: location(occurrence)
            for value in related: location(value)
        for _, application, definition, arguments, _ in wire[21][1]:
            for value in [application, definition, *arguments]: location(value)
        bases[key] = item
    for item in complete["identity_variations"]:
        base = bases[item["base"]]
        wire = edited(base["tuple"], item["wire_edits"])
        context = edited(base["context"], item["context_edits"])
        parts = project_partitions(wire, profiles, context)
        require({key: value["sha256"] for key, value in parts.items()} ==
                item["expected_digests"], "identity variation: " + item["id"])
        changed = [key for key in parts if parts[key]["sha256"] !=
                   base["partitions"][key]["sha256"]]
        require(changed == item["changed_partitions"], "identity partition exclusions")
    models = {}
    for item in authoring["valid"]:
        actual = project(item["model"])
        require(actual == {key: item["expected"][key] for key in actual},
                "authoring projection: " + item["id"])
        source = actual["source_utf8"].encode()
        require(hashlib.sha256(source).hexdigest() == item["expected"]["source_sha256"],
                "authoring source digest")
        for mapping in actual["mappings"]:
            require(0 <= mapping["start"] < mapping["end"] <= len(source), "mapping bounds")
            source[:mapping["start"]].decode("utf-8")
            source[mapping["start"]:mapping["end"]].decode("utf-8")
        models[item["id"]] = item["model"]
    for item in authoring["invalid"]:
        try:
            project(edited(models[item["base"]], [item["edit"]]))
        except ValueError as error:
            require(str(error) == item["expected_oracle_rejection"], "wrong model rejection")
        else:
            raise ValueError("invalid model accepted: " + item["id"])
