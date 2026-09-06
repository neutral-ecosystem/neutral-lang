<!-- SPDX-License-Identifier: Apache-2.0 -->

# Standards alignment register

Status: Stage 9 project alignment record; no conformity or certification claim.

| Standard | Edition | Applicability and tailoring | Evidence | Owner | Next review |
| --- | --- | --- | --- | --- | --- |
| ISO/IEC/IEEE 29119-1 | 2022 | Test concepts and vocabulary; project-scale tailoring | `portable/development/04-TESTING.md` | maintainer | Stage 10 RC |
| ISO/IEC/IEEE 29119-2 | 2021 | Stage-aware test process and gates | development stages, `cargo xtask ci pr` | maintainer | Stage 10 RC |
| ISO/IEC/IEEE 29119-3 | 2021 | Plans, cases, evidence, and result metadata | testing plan, conformance manifest, `test-results/` | maintainer | Stage 10 RC |
| ISO/IEC/IEEE 29119-4 | 2021 | Boundary, grammar, property, fault, and state techniques | crate-local and cross-package tests | maintainer | Stage 10 RC |
| ISO/IEC 25010 | 2023 | Selected product-quality characteristics | `quality-evaluation.md` | maintainer | Stage 10 RC |
| ISO/IEC 25023 | 2016 | Selected measurable quality properties | `config/quality-gates.toml` | maintainer | Stage 10 RC |
| ISO/IEC 25040 | 2024 | Evaluation method and pass/fail/indeterminate conclusions | `quality-evaluation.md` | maintainer | Stage 10 RC |
| ISO/IEC 20246 | 2017 | Static work-product review | `static-work-product-review.md` | maintainer | Stage 10 RC |
| ISO/IEC 5055 | 2021 | Selected automated source measures only | Clippy, coverage, mutation, dependency review | maintainer | Stage 10 RC |

The repository does not include the licensed standards and therefore does not
claim full conformance. Editions and applicability must be rechecked before a
release candidate is approved.
