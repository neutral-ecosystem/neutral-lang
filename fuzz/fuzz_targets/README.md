<!-- SPDX-License-Identifier: Apache-2.0 -->

# Fuzz targets

Each target drives an untrusted-input boundary with finite production controls.
Source and formatter targets capture the current expression-project contract;
IR and probe targets decode the current complete expression artifact; vocabulary
fuzzing checks current captured bundle schemas. `composition_support.rs` owns
only shared bounded expression capture and runtime-generated `/3` wire seeds.
