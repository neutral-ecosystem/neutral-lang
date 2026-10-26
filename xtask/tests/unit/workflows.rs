// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the semantic GitHub Actions workflow contract.

use crate::checks::workflows::validate_workflows;

fn ci_yaml() -> &'static str {
    r"name: CI
on:
  push:
    branches:
      - main
  workflow_dispatch: {}
permissions:
  contents: read
jobs:
  ci:
    permissions:
      contents: read
    steps:
      - run: cargo xtask ci pr
      - if: always()
        uses: actions/upload-artifact@v7
        with:
          path: test-results/workflows
          retention-days: 14
"
}

fn release_yaml() -> &'static str {
    r#"name: Release
on:
  push:
    tags:
      - 'v*'
  workflow_dispatch: {}
permissions:
  contents: read
jobs:
  qualify:
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ github.event_name == 'push' && github.ref || 'main' }}
      - if: github.event_name == 'push'
        run: |
          tag_commit="$(git rev-list -n 1 "$RELEASE_TAG")"
          main_commit="$(git rev-parse refs/remotes/origin/main)"
          test "$tag_commit" = "$main_commit"
          git switch -C main "$tag_commit"
      - if: github.event_name == 'push'
        run: test "$(cargo xtask release tag)" = "$RELEASE_TAG"
      - run: cargo xtask release qualify
      - run: sha256sum --check SHA256SUMS
      - uses: actions/upload-artifact@v7
        with:
          retention-days: 14
  publish:
    if: github.event_name == 'push'
    needs: qualify
    permissions:
      contents: write
    steps:
      - run: sha256sum -c SHA256SUMS
      - env:
          RELEASE_TAG: ${{ github.ref_name }}
        run: |
          gh release create "$RELEASE_TAG" release-assets/* \
            --verify-tag \
            --draft
"#
}

#[test]
/// Equivalent YAML structure and quoted shell formatting satisfy the workflow contract.
fn workflows_accept_equivalent_yaml_and_shell_layouts() {
    let ci = ci_yaml()
        .replace("if: always()", "if: ${{always()}}")
        .replace("branches:\n      - main", "branches: [main]")
        .replace(
            "path: test-results/workflows",
            "path: |\n            test-results/workflows\n            test-results/tests",
        );
    let release = release_yaml()
        .replace(
            "git switch -C main \"$tag_commit\"",
            "git switch -C main \"$main_commit\"",
        )
        .replace("tag_commit", "tag_hash")
        .replace("main_commit", "origin_hash")
        .replace("tags:\n      - 'v*'", "tags: ['v*']")
        .replace("git switch -C main", "git switch --force-create main")
        .replace("sha256sum --check", "sha256sum -c");
    validate_workflows(&ci, &release, "test-results/workflows").expect("workflow contract");
}

#[test]
/// Disabled checks, unsupported shell contexts and unsafe CI permissions cannot satisfy the policy.
fn ci_workflow_rejects_skipped_checks_and_unsafe_permissions() {
    validate_workflows(ci_yaml(), release_yaml(), "test-results/workflows")
        .expect("baseline workflows satisfy the contract");
    for (ci, release) in [
        (
            ci_yaml().replace(
                "- run: cargo xtask ci pr",
                "- run: |\n          run_ci() {\n            cargo xtask ci pr\n          }",
            ),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace(
                "- run: cargo xtask ci pr",
                "- run: |\n          cat <<'EOF'\n          cargo xtask ci pr\n          EOF",
            ),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace(
                "- run: cargo xtask ci pr",
                "- if: ${{false}}\n        run: cargo xtask ci pr",
            ),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace(
                "cargo xtask ci pr",
                "if false ; then cargo xtask ci pr ; fi",
            ),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace("cargo xtask ci pr", "false && cargo xtask ci pr"),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace("contents: read", "contents: write"),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace("if: always()", "if: success()"),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace("test-results/workflows", "test-results/tests"),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace("cargo xtask ci pr", "echo '$(cargo xtask ci pr)'"),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace(
                "  ci:\n    permissions:",
                "  ci:\n    if: false\n    permissions:",
            ),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace("permissions:\n  contents: read", "permissions: false"),
            release_yaml().to_owned(),
        ),
        (
            ci_yaml().replace("retention-days: 14", "retention-days: 0"),
            release_yaml().to_owned(),
        ),
    ] {
        assert!(
            validate_workflows(&ci, &release, "test-results/workflows").is_err(),
            "unsafe workflow unexpectedly passed\nCI:\n{ci}\nRelease:\n{release}"
        );
    }
}

#[test]
/// Publication requires the event tag, qualification, checksums and verified draft flags.
fn release_workflow_rejects_wrong_tags_and_missing_guards() {
    validate_workflows(ci_yaml(), release_yaml(), "test-results/workflows")
        .expect("baseline workflows satisfy the contract");
    for (ci, release) in [
        (
            ci_yaml().to_owned(),
            release_yaml().replace(
                "gh release create \"$RELEASE_TAG\"",
                "gh release create v-old",
            ),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace("RELEASE_TAG: ${{ github.ref_name }}", "RELEASE_TAG: v-old"),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace("needs: qualify", "needs: other"),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace(
                "if: github.event_name == 'push'\n    needs",
                "if: always()\n    needs",
            ),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace(
                "if: github.event_name == 'push'\n    needs",
                "if: github.event_name != 'push'\n    needs",
            ),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace("--draft", "# --draft"),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace(
                "--verify-tag \\\n            --draft",
                "--verify-tag ; echo \\\n            --draft",
            ),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace("gh release create", "echo gh release create"),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace(
                "tag_commit=\"$(git rev-list -n 1 \"$RELEASE_TAG\")\"",
                "tag_commit=ok # $(git rev-list -n 1 \"$RELEASE_TAG\")",
            ),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace(
                "git switch -C main \"$tag_commit\"",
                "git switch -C main HEAD~1",
            ),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace(
                "--verify-tag \\\n            --draft",
                "--verify-tag=false \\\n            --draft=false",
            ),
        ),
        (
            ci_yaml().to_owned(),
            release_yaml().replace("sha256sum -c SHA256SUMS", "sha256sum SHA256SUMS"),
        ),
    ] {
        assert!(
            validate_workflows(&ci, &release, "test-results/workflows").is_err(),
            "unsafe workflow unexpectedly passed\nCI:\n{ci}\nRelease:\n{release}"
        );
    }
}
