// SPDX-License-Identifier: Apache-2.0

//! Assemble encoded sections into the artifact's physical binary frame.
//!
//! This is the layout boundary: it writes the fixed header, section directory
//! and contiguous payloads while checking sizes and representable offsets.
//! Section contents arrive already encoded; framing must not reinterpret
//! logical values or change their section bytes.

use super::{EncodedArtifact, EncodingError, Range, array, constants};

/// Assembles the fixed header, directory, and contiguous sections.
pub(super) fn assemble_frame(
    capabilities: u64,
    sections: [Vec<u8>; constants::SECTION_COUNT],
) -> Result<EncodedArtifact, EncodingError> {
    let directory_bytes = constants::DIRECTORY_ENTRY_BYTES
        .checked_mul(constants::SECTION_COUNT)
        .ok_or(EncodingError::EncodedSizeLimit)?;
    let first_offset = constants::HEADER_BYTES
        .checked_add(directory_bytes)
        .ok_or(EncodingError::EncodedSizeLimit)?;
    let total_length = sections.iter().try_fold(first_offset, |length, section| {
        length
            .checked_add(section.len())
            .ok_or(EncodingError::EncodedSizeLimit)
    })?;
    if total_length > constants::MAXIMUM_ARTIFACT_BYTES {
        return Err(EncodingError::EncodedSizeLimit);
    }
    let mut ranges: [Range<usize>; constants::SECTION_COUNT] = array::from_fn(|_| 0..0);
    let mut offset = first_offset;
    for (index, section) in sections.iter().enumerate() {
        let end = offset
            .checked_add(section.len())
            .ok_or(EncodingError::EncodedSizeLimit)?;
        ranges[index] = offset..end;
        offset = end;
    }
    let mut bytes = Vec::with_capacity(total_length);
    bytes.extend_from_slice(&constants::MAGIC);
    bytes.extend_from_slice(&constants::FRAMING_REVISION.to_be_bytes());
    bytes.extend_from_slice(
        &u16::try_from(constants::HEADER_BYTES)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(total_length)
            .map_err(|_| EncodingError::EncodedSizeLimit)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(
        &u64::try_from(constants::HEADER_BYTES)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(
        &u32::try_from(constants::SECTION_COUNT)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(
        &u16::try_from(constants::DIRECTORY_ENTRY_BYTES)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&capabilities.to_be_bytes());
    for (index, range) in ranges.iter().enumerate() {
        bytes.extend_from_slice(
            &u16::try_from(index + 1)
                .map_err(|_| EncodingError::InternalDefect)?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&constants::SECTION_SCHEMA_REVISION.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(range.start)
                .map_err(|_| EncodingError::EncodedSizeLimit)?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(
            &u64::try_from(range.len())
                .map_err(|_| EncodingError::EncodedSizeLimit)?
                .to_be_bytes(),
        );
    }
    for section in sections {
        bytes.extend_from_slice(&section);
    }
    if bytes.len() != total_length {
        return Err(EncodingError::InternalDefect);
    }
    Ok(EncodedArtifact {
        bytes,
        sections: ranges,
    })
}
