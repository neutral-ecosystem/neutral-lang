<!-- SPDX-License-Identifier: Apache-2.0 -->

# Source and module contract

Status: accepted portable baseline

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
