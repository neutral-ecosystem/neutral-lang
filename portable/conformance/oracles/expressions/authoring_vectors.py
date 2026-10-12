# SPDX-License-Identifier: Apache-2.0
# Projects the registered minimal authoring models without importing production code.
"""Independent fixture oracle; deliberately rejects nodes outside its named subset."""

import json

from verify import require


def project(model):
    require(set(model) == {"schema", "units", "connections"}, "model fields")
    require(model["schema"] == "neutral.authoring-model/2", "model profile")
    require(not model["connections"] and len(model["units"]) == 1, "minimal model scope")
    unit = model["units"][0]
    require(set(unit) == {"id", "source_id", "module", "imports", "uses", "declarations",
                          "comments"}, "unit fields")
    require(not any(unit[key] for key in ("imports", "uses", "comments")), "unit scope")
    declarations = unit["declarations"]
    names = {d["id"]: d["name"] for d in declarations}
    require(len(names) == len(declarations), "duplicate declaration ID")
    seen = {unit["id"]}
    output, mappings = [], []

    def write(text):
        output.append(text)

    def offset():
        return len("".join(output).encode())

    def begin(identifier):
        require(identifier not in seen, "duplicate model ID")
        seen.add(identifier)
        return offset()

    def end(identifier, start, role):
        mappings.append({"model_id": identifier, "source_id": unit["source_id"],
                         "start": start, "end": offset(), "role": role})

    def type_name(value):
        require(len(value) == 2 and value[0] == "core" and
                value[1] in ("num", "bool", "string"), "minimal type scope")
        return value[1]

    def expression(node, binders):
        start = begin(node["id"])
        kind = node["kind"]
        common = {"id", "kind", "type"}
        if kind in ("num", "bool", "string"):
            require(set(node) == {"id", "kind", "value"}, "literal fields")
            write(node["value"] if kind == "num" else json.dumps(node["value"], ensure_ascii=False))
        elif kind == "external":
            require(set(node) == common | {"target"}, "external fields")
            target = next((d for d in declarations if d["id"] == node["target"]), None)
            require(target is not None and target["kind"] == "external-input" and
                    target["type"] == node["type"], "external target")
            write(names[node["target"]])
        elif kind == "bound":
            require(set(node) == common | {"binder"} and node["binder"] in binders,
                    "bound scope")
            write(binders[node["binder"]])
        elif kind == "binary":
            require(set(node) == common | {"operator", "left", "right"}, "binary fields")
            require(node["operator"] in ("+", "-", "*", "/"), "operator scope")
            expression(node["left"], binders)
            write(" " + node["operator"] + " ")
            expression(node["right"], binders)
        elif kind == "if":
            require(set(node) == common | {"guard", "then", "else"}, "if fields")
            write("if "); expression(node["guard"], binders)
            write(" { "); expression(node["then"], binders)
            write(" } else { "); expression(node["else"], binders); write(" }")
        elif kind == "instantiate":
            require(set(node) == common | {"target", "arguments"}, "application fields")
            target = next((d for d in declarations if d["id"] == node["target"]), None)
            require(target is not None and target["kind"] == "template", "template target")
            require([a["name"] for a in node["arguments"]] ==
                    [p["name"] for p in target["parameters"]], "argument coverage")
            write("instantiate " + target["name"] + "(")
            for index, argument in enumerate(node["arguments"]):
                require(set(argument) == {"name", "value"}, "argument fields")
                if index: write(", ")
                write(argument["name"] + ": "); expression(argument["value"], binders)
            write(")")
        else:
            raise ValueError("node outside minimal projection scope")
        end(node["id"], start, "value")

    write('neu "1.0"\nmodule ' + unit["module"] + "\n\n")
    for declaration in declarations:
        start = begin(declaration["id"])
        kind = declaration["kind"]
        common = {"id", "kind", "name", "public"}
        if declaration["public"]: write("public ")
        if kind == "external-input":
            require(set(declaration) == common | {"type"}, "input declaration fields")
            write("input " + type_name(declaration["type"]) + " " + declaration["name"])
        elif kind == "binding":
            require(set(declaration) == common | {"type", "value"}, "binding fields")
            write(type_name(declaration["type"]) + " " + declaration["name"] + " = ")
            expression(declaration["value"], {})
        elif kind == "template":
            require(set(declaration) == common | {"parameters", "result_type", "body"},
                    "template fields")
            write("template " + declaration["name"] + "(")
            binders = {}
            for index, parameter in enumerate(declaration["parameters"]):
                require(set(parameter) == {"id", "name", "phase", "type", "default"},
                        "parameter fields")
                begin(parameter["id"])
                require(parameter["phase"] in ("static", "deferred") and
                        parameter["default"] is None, "minimal parameter scope")
                if index: write(", ")
                write(parameter["phase"] + " " + type_name(parameter["type"]) + " " + parameter["name"])
                binders[parameter["id"]] = parameter["name"]
            write(") -> " + type_name(declaration["result_type"]) + " {\n    ")
            expression(declaration["body"], binders); write("\n}")
        else:
            raise ValueError("declaration outside minimal projection scope")
        end(declaration["id"], start, "declaration"); write("\n")
    return {"source_utf8": "".join(output), "mappings": sorted(mappings, key=lambda m:
            (m["source_id"], m["start"], m["end"], m["role"], m["model_id"]))}
