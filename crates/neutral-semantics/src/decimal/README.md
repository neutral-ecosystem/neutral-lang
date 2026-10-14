<!-- SPDX-License-Identifier: Apache-2.0 -->

# Decimal operations

Private base-10 arithmetic over the existing exact sign/coefficient/scale value.
`coefficient.rs` owns bounded digit operations; `division.rs` owns exact
termination detection and fixed-profile nearest/ties-even rounding. `mod.rs`
dispatches operations and canonicalizes results. No host floats are involved.

All scratch reservations, digit work and cancellation use the request's shared
budget. Normalized coefficient/scale limits remain independent of bounded
alignment and carry scratch. Operations are exposed through pure evaluation,
not as a second public arithmetic interface.
