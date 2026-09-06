<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral quality evidence

This directory owns reviewed static evidence for correctness, security,
performance, maintainability, and residual risk. Dynamic generated output stays
under ignored `test-results/`; this directory records the methods, conclusions,
owners, and follow-up decisions needed to interpret those results.

- `standards-register.md` defines the project’s alignment boundary.
- `threat-model.md` records assets, trust boundaries, threats, and controls.
- `dependency-review.md` reviews dependencies and executable build surfaces.
- `cache-isolation-review.md` reviews request/source-fact isolation.
- `static-work-product-review.md` records review coverage and findings.
- `quality-evaluation.md` evaluates the selected product characteristics.
- `residual-risks.md` prevents incomplete external campaigns from being called
  passes.
