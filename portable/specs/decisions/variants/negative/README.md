<!-- SPDX-License-Identifier: Apache-2.0 -->

# Planned invalid variants

These proposed inputs define rejection boundaries shared by source and
vocabulary variants. They belong to the future compiler/reader security corpus,
not the currently active manifest. Literal diagnostic oracles are not yet frozen.

| Input | Intended rejection |
| --- | --- |
| `duplicate-tag.neu` | Repeated alternative tag in a nominal declaration |
| `unknown-tag.neu` | Selected tag absent from the closed alternatives |
| `wrong-payload.neu` | Payload incompatible with the selected alternative |
| `missing-payload.neu` | Missing required payload is not implicit null |
| `extra-member.neu` | Variant value admits only tag and payload |
| `public-private-payload.neu` | Public type closure includes an unselected private payload type |

The equivalent hostile bundle and encoded-artifact cases must also be registered
after their new schema and wire contracts are frozen.
