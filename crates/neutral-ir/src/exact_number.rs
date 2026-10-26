// SPDX-License-Identifier: Apache-2.0

//! Exact decimal values shared by parsing, IR and identity construction.
//!
//! Numbers retain a sign, decimal coefficient and scale instead of converting
//! through floating point. Source parsing checks digit and scale budgets before
//! normalization; equivalent spellings share a normalized representation.
//! Display and hash-transcript emission consume that same representation.
//!
//! The represented value is `(-1 if negative else 1) * coefficient * 10^scale`.
//! Nonzero coefficients have no leading or trailing zeroes; zero always has a
//! positive sign and scale zero. Source parsing accepts spelling variations and
//! normalizes them, whereas decoding normalized parts rejects alternate spellings.
//! That distinction prevents multiple canonical encodings of the same value.
//!
//! This is a value representation, not the expression arithmetic policy. Decimal
//! division precision/rounding belongs to the evaluator profile. Derived `Ord`
//! compares stored fields and is not mathematical ordering; use
//! [`crate::composition::compare_exact_numbers`] for numeric comparisons.
//! Display emits a reader-facing rational notation, not source or JSON syntax.

use crate::IrError;
use neutral_core::allocation::RetainCapacity;
use neutral_core::{CoreError, nht_frame};
use std::fmt;

/// Exact normalized base-10 rational representation used by Neutral IR.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExactNumber {
    /// Whether the normalized value is negative.
    pub(super) negative: bool,
    /// Magnitude digits without separators or leading zeroes except zero itself.
    pub(super) coefficient: String,
    /// Signed base-10 scale applied to the coefficient.
    pub(super) scale: i64,
}

impl ExactNumber {
    /// Reconstructs one validated normalized exact number from external parts.
    ///
    /// The value is `sign * coefficient * 10^scale`. Nonzero coefficients have
    /// neither leading nor trailing zeroes; zero has one spelling, positive with
    /// scale zero. External decoders must reject noncanonical parts, not silently
    /// normalize them, so equivalent wire values cannot gain multiple identities.
    ///
    /// # Errors
    ///
    /// Returns an error when the coefficient is noncanonical or the configured
    /// digit or absolute-scale limit is exceeded.
    pub fn from_normalized_parts(
        negative: bool,
        coefficient: &str,
        scale: i64,
        maximum_digits: u64,
        maximum_scale: u64,
    ) -> Result<Self, IrError> {
        let digit_count = u64::try_from(coefficient.len()).unwrap_or(u64::MAX);
        if digit_count > maximum_digits || scale.unsigned_abs() > maximum_scale {
            return Err(IrError::ExactNumberLimitExceeded);
        }
        let valid_digits =
            !coefficient.is_empty() && coefficient.bytes().all(|byte| byte.is_ascii_digit());
        // Reject alternative zero encodings instead of silently repairing untrusted wire parts.
        let valid_zero = coefficient == "0" && !negative && scale == 0;
        let valid_nonzero = coefficient
            .starts_with(|character: char| ('1'..='9').contains(&character))
            && !coefficient.ends_with('0');
        if !valid_digits || (!valid_zero && !valid_nonzero) {
            return Err(IrError::InvalidExactNumber);
        }
        Ok(Self {
            negative,
            coefficient: neutral_core::allocation::text(coefficient)
                .map_err(|_| IrError::ExactNumberLimitExceeded)?,
            scale,
        })
    }

    /// Normalizes one frozen source number without floating-point conversion.
    ///
    /// Unlike the external-parts constructor, this accepts source spelling
    /// variations. Fractional digits and exponents become a decimal scale, then
    /// insignificant zeroes are removed without changing the exact value.
    /// Limits apply during parsing as well as after normalization; spelling a
    /// tiny value with excessive digits does not bypass the input-work bound.
    ///
    /// # Errors
    ///
    /// Returns an error for a malformed spelling or an exceeded digit/scale bound.
    pub fn from_source(
        spelling: &str,
        maximum_digits: u64,
        maximum_scale: u64,
    ) -> Result<Self, IrError> {
        let mut parsed = parse_source_number(spelling, maximum_digits, maximum_scale)?;
        let leading_zeroes =
            parsed.coefficient.len() - parsed.coefficient.trim_start_matches('0').len();
        if leading_zeroes == parsed.coefficient.len() {
            parsed.coefficient.clear();
            parsed.coefficient.push('0');
            // All signed/exponent spellings of zero share one canonical value.
            return Ok(Self {
                negative: false,
                coefficient: parsed.coefficient,
                scale: 0,
            });
        }
        parsed.coefficient.drain(..leading_zeroes);
        let mut coefficient = parsed.coefficient;
        let mut scale = parsed.scale;
        // Removing a coefficient zero multiplies the power of ten by one.
        while coefficient.ends_with('0') {
            coefficient.pop();
            scale = scale
                .checked_add(1)
                .ok_or(IrError::ExactNumberLimitExceeded)?;
        }
        if scale.unsigned_abs() > maximum_scale {
            return Err(IrError::ExactNumberLimitExceeded);
        }
        Ok(Self {
            negative: parsed.negative,
            coefficient,
            scale,
        })
    }

    /// Parses a number using spelling-length-derived bounds for compatibility callers.
    ///
    /// Despite the historical name, this delegates to the complete source-number
    /// grammar rather than enforcing digits-only input. Request-facing code should
    /// use [`Self::from_source`] with its independently configured numeric limits.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed source syntax or a digit/scale limit violation.
    pub fn from_unsigned_integer(spelling: &str) -> Result<Self, IrError> {
        let maximum_digits = u64::try_from(spelling.len()).unwrap_or(u64::MAX).max(1);
        Self::from_source(spelling, maximum_digits, maximum_digits)
    }

    /// Returns whether the normalized number is negative.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.negative
    }

    /// Returns normalized coefficient digits.
    #[must_use]
    pub fn coefficient(&self) -> &str {
        &self.coefficient
    }

    /// Returns the signed base-10 scale.
    #[must_use]
    pub const fn scale(&self) -> i64 {
        self.scale
    }

    /// Builds the frozen NHT logical-definition payload for this number.
    /// Uses ordinary temporary allocations; bounded identity writers frame the
    /// normalized components directly on their allocation-aware path instead.
    ///
    /// # Errors
    ///
    /// Returns an error only when fixed transcript lengths cannot be represented.
    pub fn nht_payload(&self) -> Result<Vec<u8>, CoreError> {
        let mut payload = Vec::new();
        payload.extend(nht_frame("sign", &[u8::from(self.negative)])?);
        payload.extend(nht_frame("coefficient", self.coefficient.as_bytes())?);
        payload.extend(nht_frame(
            "scale-sign",
            &[u8::from(self.scale.is_negative())],
        )?);
        payload.extend(nht_frame(
            "scale",
            self.scale.unsigned_abs().to_string().as_bytes(),
        )?);
        Ok(payload)
    }
}

/// Parsed numeric components retained only until exact normalization completes.
struct ParsedSourceNumber {
    /// Whether the source spelling carries a minus sign.
    negative: bool,
    /// Coefficient digits without separators.
    coefficient: String,
    /// Decimal scale before trailing-zero normalization.
    scale: i64,
}

/// Scans source ranges and checks digit/scale bounds before allocating the coefficient.
/// Separators contribute source bytes but not coefficient digits; the caller must
/// independently bound source length/work before invoking this scalar helper.
fn parse_source_number(
    spelling: &str,
    maximum_digits: u64,
    maximum_scale: u64,
) -> Result<ParsedSourceNumber, IrError> {
    if spelling.is_empty() || maximum_digits == 0 || maximum_scale == 0 {
        return Err(IrError::InvalidExactNumber);
    }
    let bytes = spelling.as_bytes();
    let mut index = 0;
    let negative = match bytes.first() {
        Some(b'+') => {
            index = 1;
            false
        }
        Some(b'-') => {
            index = 1;
            true
        }
        _ => false,
    };
    let integer = consume_digit_run(bytes, &mut index)?;
    let fraction = if bytes.get(index) == Some(&b'.') {
        index += 1;
        consume_digit_run(bytes, &mut index)?
    } else {
        DigitRun::empty(index)
    };
    let mut exponent = 0_i64;
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        let exponent_negative = if matches!(bytes.get(index), Some(b'+' | b'-')) {
            let value = bytes[index] == b'-';
            index += 1;
            value
        } else {
            false
        };
        let digits = consume_digit_run(bytes, &mut index)?;
        exponent = parse_bounded_exponent(
            &spelling[digits.start..digits.end],
            maximum_scale,
            exponent_negative,
        )?;
    }
    if index != bytes.len() {
        return Err(IrError::InvalidExactNumber);
    }
    let digit_count = integer.digit_count.saturating_add(fraction.digit_count);
    if u64::try_from(digit_count).unwrap_or(u64::MAX) > maximum_digits {
        return Err(IrError::ExactNumberLimitExceeded);
    }
    // Concatenating fractional digits into the integer coefficient shifts the
    // decimal point right; subtract their count from the source exponent.
    let fraction_scale =
        i64::try_from(fraction.digit_count).map_err(|_| IrError::ExactNumberLimitExceeded)?;
    let scale = exponent
        .checked_sub(fraction_scale)
        .ok_or(IrError::ExactNumberLimitExceeded)?;
    if scale.unsigned_abs() > maximum_scale {
        return Err(IrError::ExactNumberLimitExceeded);
    }
    let mut coefficient = String::new();
    // Only validated digit runs reach retention; rejecting a giant exponent needs no expanded integer.
    coefficient
        .try_retain_exact(digit_count)
        .map_err(|_| IrError::ExactNumberLimitExceeded)?;
    coefficient.extend(
        spelling[integer.start..integer.end]
            .bytes()
            .filter(u8::is_ascii_digit)
            .map(char::from),
    );
    coefficient.extend(
        spelling[fraction.start..fraction.end]
            .bytes()
            .filter(u8::is_ascii_digit)
            .map(char::from),
    );
    Ok(ParsedSourceNumber {
        negative,
        coefficient,
        scale,
    })
}

/// One validated source digit-run range without a proportional digit buffer.
struct DigitRun {
    /// First included source byte.
    start: usize,
    /// First excluded source byte.
    end: usize,
    /// Number of decimal digits excluding separators.
    digit_count: usize,
}

impl DigitRun {
    /// Creates an empty omitted fractional run at one source position.
    const fn empty(index: usize) -> Self {
        Self {
            start: index,
            end: index,
            digit_count: 0,
        }
    }
}

/// Consumes a nonempty decimal digit run with separators only between digits.
fn consume_digit_run(bytes: &[u8], index: &mut usize) -> Result<DigitRun, IrError> {
    if !bytes.get(*index).is_some_and(u8::is_ascii_digit) {
        return Err(IrError::InvalidExactNumber);
    }
    let start = *index;
    let mut digit_count = 0_usize;
    while let Some(byte) = bytes.get(*index) {
        if byte.is_ascii_digit() {
            digit_count = digit_count.saturating_add(1);
            *index += 1;
        } else if *byte == b'_' {
            if !bytes
                .get((*index).saturating_sub(1))
                .is_some_and(u8::is_ascii_digit)
                || !bytes.get(*index + 1).is_some_and(u8::is_ascii_digit)
            {
                return Err(IrError::InvalidExactNumber);
            }
            *index += 1;
        } else {
            break;
        }
    }
    Ok(DigitRun {
        start,
        end: *index,
        digit_count,
    })
}

/// Parses a bounded decimal exponent without host floating-point conversion.
/// Checks each multiply/add before applying the sign, preventing overflow or a
/// large temporary integer even when the source exponent contains many digits.
fn parse_bounded_exponent(
    digits: &str,
    maximum_scale: u64,
    negative: bool,
) -> Result<i64, IrError> {
    let mut magnitude = 0_u64;
    for byte in digits.bytes().filter(u8::is_ascii_digit) {
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            .ok_or(IrError::ExactNumberLimitExceeded)?;
        if magnitude > maximum_scale {
            return Err(IrError::ExactNumberLimitExceeded);
        }
    }
    let magnitude = i64::try_from(magnitude).map_err(|_| IrError::ExactNumberLimitExceeded)?;
    Ok(if negative { -magnitude } else { magnitude })
}

impl fmt::Display for ExactNumber {
    /// Formats the normalized exact rational used by reader and probe output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negative {
            formatter.write_str("-")?;
        }
        if self.scale == 0 {
            write!(formatter, "{}/1", self.coefficient)
        } else {
            write!(formatter, "{}e{}/1", self.coefficient, self.scale)
        }
    }
}
