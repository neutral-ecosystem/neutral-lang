// SPDX-License-Identifier: Apache-2.0

//! Stable bounded decoder diagnostic codes.

/// Encoded input exceeded an effective size or traversal ceiling.
pub const ENCODED_SIZE_LIMIT: &str = "NLE7001";
/// The fixed frame or directory was malformed.
pub const MALFORMED_FRAME: &str = "NLE7002";
/// A framing, section, encoding, or contract version is unsupported.
pub const UNSUPPORTED_VERSION: &str = "NLE7003";
/// Capability declarations are unknown, missing, unused, or inconsistent.
pub const UNSUPPORTED_CAPABILITY: &str = "NLE7004";
/// One integrity record did not match its exact section bytes.
pub const INTEGRITY_MISMATCH: &str = "NLE7005";
/// Restricted CBOR lexical or container validation failed.
pub const MALFORMED_CBOR: &str = "NLE7006";
/// A closed encoded schema had an invalid member or scalar.
pub const INVALID_ENCODED_SCHEMA: &str = "NLE7007";
/// Reconstructed logical IR failed validation.
pub const INVALID_LOGICAL_IR: &str = "NLE7008";
/// Reconstructed source-map facts failed validation.
pub const INVALID_SOURCE_MAP: &str = "NLE7009";
/// Reconstructed provenance failed validation.
pub const INVALID_PROVENANCE: &str = "NLE7010";
/// Reconstructed derivation facts failed validation.
pub const INVALID_DERIVATION: &str = "NLE7011";
/// Decoding was cancelled before immutable reader construction.
pub const CANCELLED: &str = "NLE7012";
/// A non-input implementation invariant failed.
pub const INTERNAL_DEFECT: &str = "NLE7099";
