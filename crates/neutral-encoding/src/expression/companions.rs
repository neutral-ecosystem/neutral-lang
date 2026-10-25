// SPDX-License-Identifier: Apache-2.0

//! Unchanged source companions and separate original-byte expression/expansion evidence.

use super::read_list;
use crate::{
    DecodeError as D, EncodingError as E,
    cbor::CborWriter,
    contracts::{read_option, write_option},
    decoder::LocatedValue,
    primitives::{
        read_location, read_symbol, tuple, unsigned, write_list, write_location, write_numbers,
        write_symbol,
    },
};
use neutral_core::CancellationToken;
use neutral_ir::expression::{
    attribution::{ExpansionAttribution, ExpressionAttribution},
    project::ExpressionProjectIr,
};

/// Writes separate original-byte occurrence and expansion lineage collections.
pub(super) fn write_attributions(w: &mut CborWriter, ir: &ExpressionProjectIr) -> Result<(), E> {
    w.array(2)?;
    write_list(w, &ir.expression_attributions, |w, attribution| {
        w.array(4)?;
        write_symbol(w, &attribution.owner)?;
        write_numbers(w, &attribution.node_path)?;
        write_option(w, attribution.occurrence.as_ref(), |w, location| {
            write_location(w, *location)
        })?;
        write_list(w, &attribution.related_public_locations, |w, location| {
            write_location(w, *location)
        })
    })?;
    write_list(w, &ir.expansion_attributions, |w, attribution| {
        w.array(5)?;
        write_symbol(w, &attribution.owner)?;
        write_location(w, attribution.application)?;
        write_location(w, attribution.definition)?;
        write_list(w, &attribution.arguments, |w, location| {
            write_location(w, *location)
        })?;
        write_numbers(w, &attribution.original_axis_indices)
    })
}

/// Reads child-index paths and original occurrences for independent ownership checks.
pub(super) fn read_expressions(
    v: &LocatedValue,
    cancel: &CancellationToken,
) -> Result<Vec<ExpressionAttribution>, D> {
    read_list(v, cancel, |v| {
        let a = tuple::<4>(v)?;
        Ok(ExpressionAttribution {
            owner: read_symbol(&a[0])?,
            node_path: read_list(&a[1], cancel, unsigned)?,
            occurrence: read_option(&a[2], read_location)?,
            related_public_locations: read_list(&a[3], cancel, read_location)?,
        })
    })
}

/// Retains application/definition/argument/axis order without inventing generated source.
pub(super) fn read_expansions(
    v: &LocatedValue,
    cancel: &CancellationToken,
) -> Result<Vec<ExpansionAttribution>, D> {
    read_list(v, cancel, |v| {
        let a = tuple::<5>(v)?;
        Ok(ExpansionAttribution {
            owner: read_symbol(&a[0])?,
            application: read_location(&a[1])?,
            definition: read_location(&a[2])?,
            arguments: read_list(&a[3], cancel, read_location)?,
            original_axis_indices: read_list(&a[4], cancel, unsigned)?,
        })
    })
}
