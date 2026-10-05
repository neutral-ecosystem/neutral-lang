<!-- SPDX-License-Identifier: Apache-2.0 -->

# Configuration and discovery

Typed repository configuration, Cargo metadata discovery, stable contract names,
and formatting-preserving manifest edits live here. Changeable developer defaults
belong in the repository's `config/` directory, not new Rust constants. These
modules supply inputs to commands and checks without defining language behavior.

`constants.rs` owns shared process flags, bundle-member names, and quality report
filenames. Producers and validators use the same names; runtime choices and
versioned quality thresholds remain configuration inputs, not constants.

`quality_settings.rs` loads a typed policy snapshot for each measurement or
evidence-validation invocation. Commands and validators share those settings;
human status annotations never substitute for measured acceptance. Malformed
thresholds, unsafe paths, and invalid benchmark selections fail before execution.
