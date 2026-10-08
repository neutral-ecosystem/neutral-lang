# SPDX-License-Identifier: Apache-2.0
"""Independent public /2 projection; the accepted complete-identity oracle stays immutable."""

import json
import sys

import reference


def inspect(request):
    """Project public contracts only; roots and occurrence evidence cannot select this interface."""
    project = dict(request["logical"])
    project["declarations"] = [d for d in project["declarations"] if d["public"]]
    project["definitions"] = [d for d in project["definitions"] if d["public"]]
    return reference.encode(request, "interface", lambda e: e.logical(project))


if __name__ == "__main__":
    print(json.dumps(inspect(json.load(sys.stdin)), indent=2))
