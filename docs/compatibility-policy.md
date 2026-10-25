<!-- SPDX-License-Identifier: Apache-2.0 -->

# Development compatibility policy

Before the first stable release, this checkout targets one current language
pipeline. The approved [latest-only design](superpowers/specs/2026-10-24-latest-only-development-design.md)
supersedes earlier development requirements to keep historical source profiles
and complete project codecs executable.

Current source uses `neu "1.0"`. Complete capture, project IR, artifact, view and
identity use their existing `/3` selectors. Expression, evaluator, expansion and
decimal `/1` profiles and vocabulary composition schema `2.0` are independent
current contracts; a smaller number does not make a contract obsolete.

Old and unknown source/artifact selectors must reject explicitly. There is no
fallback, relabeling of bytes or automatic artifact conversion. Frozen historical
specifications, fixtures, vectors and evidence remain records of their original
releases, rather than acceptance requirements for the development implementation.
Current frozen wire and identity vectors remain unchanged.

The cutover is not a stable release or a waiver of safety and quality gates.
Bounds, cancellation, fallible retention, atomic publication, privacy, independent
reading and compiler-free probing still apply. Measurements from earlier inputs
do not qualify the changed tree. Follow the [implementation plan](superpowers/plans/2026-10-24-latest-only-development.md)
for the cutover's actual completion state.

At the first stable release, freeze the supported source, vocabulary, artifact,
public API and interpretation contracts as the compatibility baseline. Subsequent
changes must preserve that baseline or follow an explicitly reviewed breaking
release policy. Stable compatibility does not imply support for unknown future
features or profiles.
