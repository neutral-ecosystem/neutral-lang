# Neutral language v0 — portable implementation seed

This directory is a self-contained starting point for the `neutral-lang`
implementation repository. Copy the contents of this directory, not the
directory itself, into the new repository root.

Start with [DEVELOPMENT.md](DEVELOPMENT.md). Do not implement language behavior
until the mandatory [contract-freeze gate](development/CONTRACT-FREEZE.md) is
approved.

## Layout

```text
DEVELOPMENT.md                operational entry point
ROADMAP.md                    milestone summary
development/                  implementation, testing, automation, release rules
spec/v0/                      normative v0 baseline and decisions
conformance/                  future conformance manifests and execution assets
spec/v0/fixtures/             accepted positive and negative source fixtures
docs/                         copied language showcase and supporting reading
```

`development/PROGRESS.md` is the solo-maintained working log. It records only
current focus, immediate actions, blockers, and completed validation evidence;
the specifications and checklists remain acceptance criteria rather than a
second mutable status system.
