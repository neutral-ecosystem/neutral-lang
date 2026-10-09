// SPDX-License-Identifier: Apache-2.0

//! release boundary for repository automation.

pub(crate) mod approval;
pub(crate) mod lifecycle;
pub(crate) mod plan;
pub(crate) mod release_metadata;
pub(crate) mod version_update;
pub(crate) use plan::*;
