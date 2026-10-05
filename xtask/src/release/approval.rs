// SPDX-License-Identifier: Apache-2.0

//! release / approval responsibilities for repository automation.

use crate::{
    BTreeSet, Path, configuration_value, constants, read_quality_approvals, read_workspace_text,
    release, require_main_head_checkout, sha256_file, workspace_package_version, workspace_root,
};

/// Reads and verifies the explicit release-authority selection for a `main`-head candidate.
pub(crate) fn release_plan() -> Result<release::ReleasePlan, String> {
    let root = workspace_root()?;
    let package_version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    let plan =
        release::ReleasePlan::read(&root.join(constants::RELEASE_CONFIG_FILE), &package_version)?;
    let residual_risk_status = quality_document_status(&root, constants::RESIDUAL_RISKS_FILE)?;
    if !matches!(
        residual_risk_status.as_str(),
        "approved" | "approved-with-limitation"
    ) {
        return Err(format!(
            "residual-risk review is not approved in the quality manifest; found {residual_risk_status}"
        ));
    }
    if plan
        .channels
        .contains(&release::DistributionChannel::GithubBinaries)
    {
        let expected = BTreeSet::from([
            constants::NEUTRAL_CLI.to_owned(),
            constants::NEUTRAL_PROBE.to_owned(),
        ]);
        let selected = plan.binaries.iter().cloned().collect::<BTreeSet<_>>();
        if selected != expected {
            return Err(format!(
                "GitHub binary scope must select exactly {expected:?}; found {selected:?}"
            ));
        }
    }
    Ok(plan)
}

/// Reads the lifecycle status of one durable document from the quality manifest.
pub(crate) fn quality_document_status(root: &Path, document: &str) -> Result<String, String> {
    let manifest = read_workspace_text(root, constants::QUALITY_MANIFEST_FILE)?;
    for entry in manifest.split("[[document]]").skip(1) {
        if configuration_value(entry, "path").as_deref() == Some(document) {
            return configuration_value(entry, "status")
                .ok_or_else(|| format!("quality document {document} has no status"));
        }
    }
    Err(format!(
        "quality document is not registered in the manifest: {document}"
    ))
}

/// Requires current main to descend from the approved candidate under the same quality policy.
pub(crate) fn verify_release_approval() -> Result<(), String> {
    let plan = release_plan()?;
    let approvals = read_quality_approvals()?;
    let approval = approvals
        .iter()
        .find(|record| record.release == plan.release_tag)
        .ok_or_else(|| {
            format!(
                "release {} has no quality approval record",
                plan.release_tag
            )
        })?;
    if approval.status != "approved" {
        return Err(format!(
            "release {} quality approval is not approved",
            plan.release_tag
        ));
    }
    let root = workspace_root()?;
    if approval.quality_gates_sha256 != sha256_file(&root.join(constants::QUALITY_GATES_FILE))? {
        return Err("quality gates changed after release approval".to_owned());
    }
    let head = require_main_head_checkout()?;
    release::verify_approval_lineage(&root, &approval.commit, &head)
}
