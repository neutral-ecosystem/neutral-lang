<!-- SPDX-License-Identifier: Apache-2.0 -->

# Source and module contract

Status: accepted portable baseline

## Profile selection and Stage 1 availability

The source header selects one exact profile without normalization. `neu "0.1"`
selects the frozen, available v0.1 behavior. `neu "1.0"` is a recognized v1
profile but remains unavailable until its active stages implement the required
project behavior; Stage 1 rejects it with `NEU-PRO-001` before parsing any v1
body. Unknown or escaped lookalikes retain `NEU-SYN-002` and cannot fall back to
another profile.

The public profile catalogue is the compatibility authority. An unavailable
profile reports no implemented capabilities. Shared structural limits apply to
both selection and subsequent processing; exceeding a limit fails closed and
never selects a different profile. Existing `neu "0.1"` source and artifacts
require no migration and are never reinterpreted as v1.

No v1 profile grants functions, control flow, mutation, effects, acquisition,
secrets, packages, product commands, executable vocabularies, or runtime
meaning. Until later stages explicitly activate source shapes, all such v1
inputs fail at the unavailable-profile gate.

Every v1 source unit starts with `neu "1.0"` and exactly one qualified logical
`module` header. A module name is a non-empty `snake_case` sequence separated
by `::`; it is not a path and is not derived from a file name.

```neu
neu "1.0"
module example::shared

public string title = "neutral"
```

There is exactly one source unit per logical module. After headers and `use`
requirements, an import has a required local alias:

```neu
import example::shared as shared
public string title = shared::title
```

Aliases are unique across imports and vocabulary requirements. Imports are
logical-module references inside the supplied captured closure only: no paths,
URLs, wildcard/relative/implicit imports, inferred aliases, source-selected
versions, partial modules, or re-exports exist in v1.

Modules form an import graph. Import SCCs are valid and analyzed as a group;
only illegal declaration/value/embedded-record semantic cycles fail. Input and
declaration order cannot affect accepted meaning or diagnostic order.

Declarations are private unless prefixed `public`. Imported modules can see
only public names. A public signature must use only public transitively
reachable types. A public value may reuse a private value, but every exposed
`Ref<T>` must target a public binding. Visibility is language API shape, not
authorization or effect permission.
