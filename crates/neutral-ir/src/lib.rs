// SPDX-License-Identifier: Apache-2.0

//! Public logical representation contracts for Neutral artifacts.
//!
//! This crate owns the logical IR, source maps, provenance, and derivation
//! records. It must not acquire source input, expose compiler-private models, or
//! perform host I/O.

pub mod language;

use neutral_core::{
    ByteSpan, CoreError, SemanticDigest, SourceContentDigest, StructuralLimits,
    VocabularyContentDigest, nht_frame,
};
use std::{collections::BTreeMap, fmt};

/// Frozen logical IR schema version for the minimal v0 artifact.
pub const LOGICAL_IR_SCHEMA_VERSION: &str = "0.1.0";
/// Frozen v0 language-behavior version shared by artifacts and producers.
pub const LANGUAGE_BEHAVIOR_VERSION: &str = "0.1.0";
/// Frozen source-map schema version for the minimal v0 artifact.
pub const SOURCE_MAP_VERSION: &str = "0.1.0";
/// Frozen provenance schema version for the minimal v0 artifact.
pub const PROVENANCE_VERSION: &str = "0.1.0";
/// Frozen derivation schema version for the minimal v0 artifact.
pub const DERIVATION_SCHEMA_VERSION: &str = "0.1.0";

/// Structured logical identity of one Neutral module.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogicalModuleIdentity {
    /// Frozen language behavior version.
    language_behavior_version: String,
    /// Validated logical module name.
    module_name: String,
}

impl LogicalModuleIdentity {
    /// Creates a logical module identity from validated semantic values.
    #[must_use]
    pub fn new(
        language_behavior_version: impl Into<String>,
        module_name: impl Into<String>,
    ) -> Self {
        Self {
            language_behavior_version: language_behavior_version.into(),
            module_name: module_name.into(),
        }
    }

    /// Returns the frozen language behavior version.
    #[must_use]
    pub fn language_behavior_version(&self) -> &str {
        &self.language_behavior_version
    }

    /// Returns the validated logical module name.
    #[must_use]
    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}

/// Exact nominal identity of one user record type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NominalTypeIdentity {
    /// Logical module that owns the record declaration.
    module: LogicalModuleIdentity,
    /// Validated uppercase-leading record name.
    name: String,
}

impl NominalTypeIdentity {
    /// Creates one module-owned nominal record identity.
    #[must_use]
    pub fn new(module: LogicalModuleIdentity, name: impl Into<String>) -> Self {
        Self {
            module,
            name: name.into(),
        }
    }

    /// Returns the module that owns this nominal type.
    #[must_use]
    pub const fn module(&self) -> &LogicalModuleIdentity {
        &self.module
    }

    /// Returns the validated record type name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Structured continuity identity for one declaration in a logical module.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleSymbolIdentity {
    /// Logical module that owns the declaration.
    module: LogicalModuleIdentity,
    /// Validated declaration source name.
    declaration_name: String,
}

impl ModuleSymbolIdentity {
    /// Creates a module-symbol identity from validated semantic values.
    #[must_use]
    pub fn new(module: LogicalModuleIdentity, declaration_name: impl Into<String>) -> Self {
        Self {
            module,
            declaration_name: declaration_name.into(),
        }
    }

    /// Returns the owning logical module identity.
    #[must_use]
    pub const fn module(&self) -> &LogicalModuleIdentity {
        &self.module
    }

    /// Returns the declaration name used for continuity.
    #[must_use]
    pub fn declaration_name(&self) -> &str {
        &self.declaration_name
    }
}

/// An opaque graph-local element label without cross-document meaning.
///
/// This label is valid only inside its owning logical document. Persist
/// [`ModuleSymbolIdentity`] for continuity; never serialize or compare this
/// numeric spelling as a durable external identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementId(u64);

impl ElementId {
    /// Creates a validated-document-local element identifier.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the local integer label for indexing within one document only.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Exact normalized base-10 rational representation used by Neutral IR.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExactNumber {
    /// Whether the normalized value is negative.
    negative: bool,
    /// Magnitude digits without separators or leading zeroes except zero itself.
    coefficient: String,
    /// Signed base-10 scale applied to the coefficient.
    scale: i64,
}

impl ExactNumber {
    /// Reconstructs one validated normalized exact number from external parts.
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
        let valid_zero = coefficient == "0" && !negative && scale == 0;
        let valid_nonzero = coefficient
            .starts_with(|character: char| ('1'..='9').contains(&character))
            && !coefficient.ends_with('0');
        if !valid_digits || (!valid_zero && !valid_nonzero) {
            return Err(IrError::InvalidExactNumber);
        }
        Ok(Self {
            negative,
            coefficient: coefficient.to_owned(),
            scale,
        })
    }

    /// Normalizes one frozen source number without floating-point conversion.
    ///
    /// # Errors
    ///
    /// Returns an error for a malformed spelling or an exceeded digit/scale bound.
    pub fn from_source(
        spelling: &str,
        maximum_digits: u64,
        maximum_scale: u64,
    ) -> Result<Self, IrError> {
        let parsed = parse_source_number(spelling, maximum_digits, maximum_scale)?;
        let coefficient = parsed.coefficient.trim_start_matches('0');
        if coefficient.is_empty() {
            return Ok(Self {
                negative: false,
                coefficient: "0".to_owned(),
                scale: 0,
            });
        }
        let mut coefficient = coefficient.to_owned();
        let mut scale = parsed.scale;
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

    /// Normalizes one unsigned digits-only integer for minimal compatibility tests.
    ///
    /// # Errors
    ///
    /// Returns an error when the spelling is empty or contains non-digits.
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

/// Parses the frozen numeric grammar before allocating its normalized coefficient.
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
    let fraction_scale =
        i64::try_from(fraction.digit_count).map_err(|_| IrError::ExactNumberLimitExceeded)?;
    let scale = exponent
        .checked_sub(fraction_scale)
        .ok_or(IrError::ExactNumberLimitExceeded)?;
    if scale.unsigned_abs() > maximum_scale {
        return Err(IrError::ExactNumberLimitExceeded);
    }
    let mut coefficient = String::with_capacity(digit_count);
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

/// Exact captured and logical identity of one vocabulary contract.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyIdentity {
    /// Logical vocabulary name used by source qualification.
    identity: String,
    /// Exact vocabulary release version.
    version: String,
    /// Exact logical vocabulary schema version.
    schema_version: String,
    /// Exact captured bundle encoding version.
    encoding_version: String,
    /// Digest of the exact captured bundle bytes.
    content_digest: VocabularyContentDigest,
    /// Required immutable structural feature IDs in canonical order.
    required_features: Vec<String>,
}

impl VocabularyIdentity {
    /// Creates an exact validated vocabulary identity contract.
    #[must_use]
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        schema_version: impl Into<String>,
        encoding_version: impl Into<String>,
        content_digest: VocabularyContentDigest,
        required_features: Vec<String>,
    ) -> Self {
        Self {
            identity: identity.into(),
            version: version.into(),
            schema_version: schema_version.into(),
            encoding_version: encoding_version.into(),
            content_digest,
            required_features,
        }
    }

    /// Returns the logical vocabulary name.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// Returns the exact release version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
    /// Returns the logical schema version.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }
    /// Returns the captured encoding version.
    #[must_use]
    pub fn encoding_version(&self) -> &str {
        &self.encoding_version
    }
    /// Returns the exact captured-byte digest.
    #[must_use]
    pub const fn content_digest(&self) -> VocabularyContentDigest {
        self.content_digest
    }
    /// Returns canonical required structural feature IDs.
    #[must_use]
    pub fn required_features(&self) -> &[String] {
        &self.required_features
    }
}

/// Exact qualified identity of one vocabulary-owned nominal type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyTypeIdentity {
    /// Logical vocabulary namespace.
    vocabulary: String,
    /// Vocabulary-owned nominal type name.
    name: String,
}

impl VocabularyTypeIdentity {
    /// Creates a validated qualified vocabulary type identity.
    #[must_use]
    pub fn new(vocabulary: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            vocabulary: vocabulary.into(),
            name: name.into(),
        }
    }
    /// Returns the vocabulary namespace.
    #[must_use]
    pub fn vocabulary(&self) -> &str {
        &self.vocabulary
    }
    /// Returns the vocabulary-owned type name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for VocabularyTypeIdentity {
    /// Formats the stable qualified source type.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}::{}", self.vocabulary, self.name)
    }
}

/// Resolved minimal Neutral type identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResolvedType {
    /// Exact Neutral numeric type.
    Num,
    /// Unicode scalar-sequence string type.
    String,
    /// Exact Boolean type.
    Bool,
    /// Exact module-owned nominal record type.
    Record(NominalTypeIdentity),
    /// Exact nominal record type owned by a captured vocabulary.
    VocabularyRecord(VocabularyTypeIdentity),
    /// Ordered homogeneous list with one invariant element type.
    List(Box<ResolvedType>),
    /// Typed document-local identity reference with one invariant target type.
    Ref(Box<ResolvedType>),
    /// One outer nullable layer around an otherwise resolved type.
    Nullable(Box<ResolvedType>),
}

impl ResolvedType {
    /// Wraps a resolved non-null type in exactly one outer nullable layer.
    #[must_use]
    pub fn nullable(inner: Self) -> Self {
        Self::Nullable(Box::new(inner))
    }

    /// Wraps one resolved type as an invariant ordered list element type.
    #[must_use]
    pub fn list(inner: Self) -> Self {
        Self::List(Box::new(inner))
    }

    /// Wraps one exact resolved target type as a document-local identity reference.
    #[must_use]
    pub fn reference(inner: Self) -> Self {
        Self::Ref(Box::new(inner))
    }

    /// Returns whether this type has an outer nullable layer.
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        matches!(self, Self::Nullable(_))
    }

    /// Returns the inner resolved type when this type is nullable.
    #[must_use]
    pub fn nullable_inner(&self) -> Option<&Self> {
        match self {
            Self::Nullable(inner) => Some(inner),
            _ => None,
        }
    }

    /// Returns whether a logical value satisfies exact identity plus outer nullability.
    #[must_use]
    pub fn accepts_value(&self, value: &LogicalValue) -> bool {
        match (self, value) {
            (Self::Num, LogicalValue::Number(_))
            | (Self::String, LogicalValue::String(_))
            | (Self::Bool, LogicalValue::Boolean(_))
            | (Self::Nullable(_), LogicalValue::Null) => true,
            (Self::Record(expected), LogicalValue::Record(value)) => {
                expected == value.nominal_type()
            }
            (Self::VocabularyRecord(expected), LogicalValue::VocabularyRecord(value)) => {
                expected == value.nominal_type()
            }
            (Self::List(expected), LogicalValue::List(items)) => {
                items.iter().all(|item| expected.accepts_value(item))
            }
            (Self::Ref(expected), LogicalValue::Reference(reference)) => {
                expected.as_ref() == reference.target_type()
            }
            (Self::Nullable(inner), value) => inner.accepts_value(value),
            _ => false,
        }
    }
}

impl fmt::Display for ResolvedType {
    /// Formats the stable source-facing type name.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num => formatter.write_str("num"),
            Self::String => formatter.write_str("string"),
            Self::Bool => formatter.write_str("bool"),
            Self::Record(identity) => formatter.write_str(identity.name()),
            Self::VocabularyRecord(identity) => identity.fmt(formatter),
            Self::List(inner) => write!(formatter, "List<{inner}>"),
            Self::Ref(inner) => write!(formatter, "Ref<{inner}>"),
            Self::Nullable(inner) => write!(formatter, "{inner}?"),
        }
    }
}

/// Immutable logical value in the active public IR slice.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LogicalValue {
    /// Exact normalized Neutral number.
    Number(ExactNumber),
    /// Exact decoded Unicode scalar sequence.
    String(String),
    /// Exact Boolean value.
    Boolean(bool),
    /// Explicit null value, valid only with a nullable resolved type.
    Null,
    /// Contextually typed nominal record value with canonical field order.
    Record(RecordValue),
    /// Contextually typed vocabulary-owned record value.
    VocabularyRecord(VocabularyRecordValue),
    /// Ordered homogeneous logical values.
    List(Vec<LogicalValue>),
    /// Typed document-local identity edge to one binding declaration.
    Reference(IdentityReference),
}

impl fmt::Display for LogicalValue {
    /// Formats a deterministic reader-facing logical value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(formatter),
            Self::String(value) => format_safe_string(value, formatter),
            Self::Boolean(value) => value.fmt(formatter),
            Self::Null => formatter.write_str("null"),
            Self::Record(value) => value.fmt(formatter),
            Self::VocabularyRecord(value) => value.fmt(formatter),
            Self::List(items) => {
                formatter.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    item.fmt(formatter)?;
                }
                formatter.write_str("]")
            }
            Self::Reference(reference) => {
                write!(formatter, "ref(#{})", reference.target_element_id().get())
            }
        }
    }
}

/// One final vocabulary-owned contextual record value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyRecordValue {
    /// Exact qualified nominal type supplied by context.
    nominal_type: VocabularyTypeIdentity,
    /// Final fields in canonical name order.
    fields: Vec<RecordValueField>,
}

impl VocabularyRecordValue {
    /// Creates one validated vocabulary-owned record value.
    #[must_use]
    pub fn new(nominal_type: VocabularyTypeIdentity, fields: Vec<RecordValueField>) -> Self {
        Self {
            nominal_type,
            fields,
        }
    }
    /// Returns the exact qualified nominal type.
    #[must_use]
    pub const fn nominal_type(&self) -> &VocabularyTypeIdentity {
        &self.nominal_type
    }
    /// Returns final fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[RecordValueField] {
        &self.fields
    }
}

impl fmt::Display for VocabularyRecordValue {
    /// Formats a deterministic generic reader-facing value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("{")?;
        for (index, field) in self.fields.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            write!(formatter, "{}: {}", field.name(), field.value())?;
        }
        formatter.write_str("}")
    }
}

/// One typed document-local identity edge retained in logical IR.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdentityReference {
    /// Authoritative document-local target binding identifier.
    element_id: ElementId,
    /// Durable target identity used by fingerprints instead of the local ID.
    symbol_identity: ModuleSymbolIdentity,
    /// Redundant exact target-type constraint validated against the declaration.
    resolved_type: ResolvedType,
}

impl IdentityReference {
    /// Creates one typed identity edge after target validation.
    #[must_use]
    pub fn new(
        target_element_id: ElementId,
        target_symbol_identity: ModuleSymbolIdentity,
        target_type: ResolvedType,
    ) -> Self {
        Self {
            element_id: target_element_id,
            symbol_identity: target_symbol_identity,
            resolved_type: target_type,
        }
    }

    /// Returns the authoritative document-local target identifier.
    #[must_use]
    pub const fn target_element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the target's module-symbol continuity identity.
    #[must_use]
    pub const fn target_symbol_identity(&self) -> &ModuleSymbolIdentity {
        &self.symbol_identity
    }

    /// Returns the redundant exact target-type constraint.
    #[must_use]
    pub const fn target_type(&self) -> &ResolvedType {
        &self.resolved_type
    }
}

/// One final field in a contextual record value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordValueField {
    /// Validated field name.
    name: String,
    /// Final recursively lowered logical value.
    value: LogicalValue,
}

impl RecordValueField {
    /// Creates one validated final record field.
    #[must_use]
    pub fn new(name: impl Into<String>, value: LogicalValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }

    /// Returns the validated field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the final field value.
    #[must_use]
    pub const fn value(&self) -> &LogicalValue {
        &self.value
    }
}

/// One contextually typed nominal record value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordValue {
    /// Exact nominal type supplied by context.
    nominal_type: NominalTypeIdentity,
    /// Final fields sorted by field name.
    fields: Vec<RecordValueField>,
}

impl RecordValue {
    /// Creates one validated contextual record value.
    #[must_use]
    pub fn new(nominal_type: NominalTypeIdentity, fields: Vec<RecordValueField>) -> Self {
        Self {
            nominal_type,
            fields,
        }
    }

    /// Returns the exact contextual nominal type.
    #[must_use]
    pub const fn nominal_type(&self) -> &NominalTypeIdentity {
        &self.nominal_type
    }

    /// Returns final fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[RecordValueField] {
        &self.fields
    }
}

impl fmt::Display for RecordValue {
    /// Formats a deterministic reader-facing record value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("{")?;
        for (index, field) in self.fields.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            write!(formatter, "{}: {}", field.name(), field.value())?;
        }
        formatter.write_str("}")
    }
}

/// Formats a logical string with all hostile control text escaped.
fn format_safe_string(value: &str, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("\"")?;
    for character in value.chars() {
        match character {
            '"' => formatter.write_str("\\\"")?,
            '\\' => formatter.write_str("\\\\")?,
            '\n' => formatter.write_str("\\n")?,
            '\r' => formatter.write_str("\\r")?,
            '\t' => formatter.write_str("\\t")?,
            '\0' => formatter.write_str("\\0")?,
            character if character.is_control() => {
                write!(formatter, "\\u{{{:x}}}", u32::from(character))?;
            }
            character => write!(formatter, "{character}")?,
        }
    }
    formatter.write_str("\"")
}

/// Frozen declaration fingerprint separated from module-symbol continuity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclarationFingerprint(SemanticDigest);

impl DeclarationFingerprint {
    /// Reconstructs a declared fingerprint from validated external digest bytes.
    #[must_use]
    pub const fn from_digest(digest: SemanticDigest) -> Self {
        Self(digest)
    }

    /// Computes the v1 binding fingerprint over resolved type and logical value.
    ///
    /// # Errors
    ///
    /// Returns an error only when NHT framing exceeds its fixed widths.
    pub fn for_binding(
        resolved_type: &ResolvedType,
        value: &LogicalValue,
    ) -> Result<Self, CoreError> {
        let mut payload = Vec::new();
        payload.extend(nht_frame("declaration-kind", b"binding")?);
        payload.extend(nht_frame(
            "resolved-type",
            resolved_type.to_string().as_bytes(),
        )?);
        let definition = logical_value_payload(value)?;
        payload.extend(nht_frame("logical-definition", &definition)?);
        SemanticDigest::from_nht("neutral/declaration-fingerprint/v1", &payload).map(Self)
    }

    /// Computes the v1 record-schema fingerprint over canonical field contracts.
    ///
    /// # Errors
    ///
    /// Returns an error only when NHT framing exceeds its fixed widths.
    pub fn for_record(fields: &[RecordFieldSchema]) -> Result<Self, CoreError> {
        let mut payload = Vec::new();
        payload.extend(nht_frame("declaration-kind", b"record")?);
        for field in fields {
            let mut definition = Vec::new();
            definition.extend(nht_frame("field-name", field.name().as_bytes())?);
            definition.extend(nht_frame(
                "field-type",
                field.resolved_type().to_string().as_bytes(),
            )?);
            let default = match field.default_value() {
                Some(value) => logical_value_payload(value)?,
                None => nht_frame("required", &[])?,
            };
            definition.extend(nht_frame("field-default", &default)?);
            payload.extend(nht_frame("record-field", &definition)?);
        }
        SemanticDigest::from_nht("neutral/declaration-fingerprint/v1", &payload).map(Self)
    }

    /// Returns the underlying domain-separated semantic digest.
    #[must_use]
    pub const fn digest(self) -> SemanticDigest {
        self.0
    }
}

/// Builds the recursive NHT payload for one final logical value.
fn logical_value_payload(value: &LogicalValue) -> Result<Vec<u8>, CoreError> {
    match value {
        LogicalValue::Number(number) => number.nht_payload(),
        LogicalValue::String(value) => nht_frame("string", value.as_bytes()),
        LogicalValue::Boolean(value) => nht_frame("boolean", &[u8::from(*value)]),
        LogicalValue::Null => nht_frame("null", &[]),
        LogicalValue::Record(value) => {
            let mut payload = Vec::new();
            payload.extend(nht_frame(
                "record-type",
                value.nominal_type().name().as_bytes(),
            )?);
            for field in value.fields() {
                let mut definition = Vec::new();
                definition.extend(nht_frame("field-name", field.name().as_bytes())?);
                definition.extend(nht_frame(
                    "field-value",
                    &logical_value_payload(field.value())?,
                )?);
                payload.extend(nht_frame("record-field", &definition)?);
            }
            nht_frame("record", &payload)
        }
        LogicalValue::VocabularyRecord(value) => {
            let mut payload = nht_frame(
                "qualified-type",
                value.nominal_type().to_string().as_bytes(),
            )?;
            for field in value.fields() {
                let mut framed = nht_frame("field-name", field.name().as_bytes())?;
                framed.extend(nht_frame(
                    "field-value",
                    &logical_value_payload(field.value())?,
                )?);
                payload.extend(nht_frame("field", &framed)?);
            }
            nht_frame("vocabulary-record", &payload)
        }
        LogicalValue::List(items) => {
            let mut payload = Vec::new();
            for item in items {
                payload.extend(nht_frame("list-item", &logical_value_payload(item)?)?);
            }
            nht_frame("list", &payload)
        }
        LogicalValue::Reference(reference) => {
            let mut payload = Vec::new();
            payload.extend(nht_frame(
                "target-language-version",
                reference
                    .target_symbol_identity()
                    .module()
                    .language_behavior_version()
                    .as_bytes(),
            )?);
            payload.extend(nht_frame(
                "target-module",
                reference
                    .target_symbol_identity()
                    .module()
                    .module_name()
                    .as_bytes(),
            )?);
            payload.extend(nht_frame(
                "target-declaration",
                reference
                    .target_symbol_identity()
                    .declaration_name()
                    .as_bytes(),
            )?);
            payload.extend(nht_frame(
                "target-type",
                reference.target_type().to_string().as_bytes(),
            )?);
            nht_frame("identity-reference", &payload)
        }
    }
}

/// One typed field contract in a nominal record declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordFieldSchema {
    /// Validated field name.
    name: String,
    /// Fully resolved field type.
    resolved_type: ResolvedType,
    /// Final closed default value, or `None` when the field is required.
    default_value: Option<LogicalValue>,
}

impl RecordFieldSchema {
    /// Creates one validated required record field.
    #[must_use]
    pub fn new(name: impl Into<String>, resolved_type: ResolvedType) -> Self {
        Self {
            name: name.into(),
            resolved_type,
            default_value: None,
        }
    }

    /// Attaches one validated closed logical default to this field contract.
    #[must_use]
    pub fn with_default(mut self, default_value: LogicalValue) -> Self {
        self.default_value = Some(default_value);
        self
    }

    /// Returns the validated field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the fully resolved required field type.
    #[must_use]
    pub const fn resolved_type(&self) -> &ResolvedType {
        &self.resolved_type
    }

    /// Returns the final closed default, or `None` when this field is required.
    #[must_use]
    pub const fn default_value(&self) -> Option<&LogicalValue> {
        self.default_value.as_ref()
    }

    /// Returns whether this field must be supplied explicitly.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.default_value.is_none()
    }
}

/// One exported nominal record type declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordTypeDefinition {
    /// Graph-local declaration identifier.
    element_id: ElementId,
    /// Durable module-symbol continuity identity.
    symbol_identity: ModuleSymbolIdentity,
    /// Logical schema fingerprint.
    fingerprint: DeclarationFingerprint,
    /// Exact nominal record identity.
    nominal_identity: NominalTypeIdentity,
    /// Required fields in canonical name order.
    fields: Vec<RecordFieldSchema>,
}

impl RecordTypeDefinition {
    /// Creates one fully validated nominal record declaration.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        symbol_identity: ModuleSymbolIdentity,
        fingerprint: DeclarationFingerprint,
        nominal_identity: NominalTypeIdentity,
        fields: Vec<RecordFieldSchema>,
    ) -> Self {
        Self {
            element_id,
            symbol_identity,
            fingerprint,
            nominal_identity,
            fields,
        }
    }

    /// Returns the graph-local declaration identifier.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the module-symbol continuity identity.
    #[must_use]
    pub const fn symbol_identity(&self) -> &ModuleSymbolIdentity {
        &self.symbol_identity
    }

    /// Returns the logical schema fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> DeclarationFingerprint {
        self.fingerprint
    }

    /// Returns the exact nominal identity.
    #[must_use]
    pub const fn nominal_identity(&self) -> &NominalTypeIdentity {
        &self.nominal_identity
    }

    /// Returns the validated record name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.nominal_identity.name()
    }

    /// Returns required fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[RecordFieldSchema] {
        &self.fields
    }
}

/// One exported immutable logical declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    /// Graph-local identifier.
    element_id: ElementId,
    /// Durable module-symbol continuity identity.
    symbol_identity: ModuleSymbolIdentity,
    /// Logical definition fingerprint.
    fingerprint: DeclarationFingerprint,
    /// Validated source declaration name.
    name: String,
    /// Resolved declared type.
    resolved_type: ResolvedType,
    /// Final immutable logical value.
    value: LogicalValue,
}

impl Declaration {
    /// Creates one fully validated logical binding declaration.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        symbol_identity: ModuleSymbolIdentity,
        fingerprint: DeclarationFingerprint,
        name: impl Into<String>,
        resolved_type: ResolvedType,
        value: LogicalValue,
    ) -> Self {
        Self {
            element_id,
            symbol_identity,
            fingerprint,
            name: name.into(),
            resolved_type,
            value,
        }
    }

    /// Returns the graph-local element identifier.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the module-symbol continuity identity.
    #[must_use]
    pub const fn symbol_identity(&self) -> &ModuleSymbolIdentity {
        &self.symbol_identity
    }

    /// Returns the logical definition fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> DeclarationFingerprint {
        self.fingerprint
    }

    /// Returns the validated source name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the resolved type.
    #[must_use]
    pub const fn resolved_type(&self) -> &ResolvedType {
        &self.resolved_type
    }

    /// Returns the final immutable logical value.
    #[must_use]
    pub const fn value(&self) -> &LogicalValue {
        &self.value
    }
}

/// One immutable field in a captured vocabulary type contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyFieldContract {
    /// Canonical field name.
    name: String,
    /// Fully resolved field type.
    resolved_type: ResolvedType,
    /// Final closed vocabulary default, when present.
    default_value: Option<LogicalValue>,
}

impl VocabularyFieldContract {
    /// Creates one validated vocabulary field contract.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        resolved_type: ResolvedType,
        default_value: Option<LogicalValue>,
    ) -> Self {
        Self {
            name: name.into(),
            resolved_type,
            default_value,
        }
    }
    /// Returns the canonical field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the fully resolved field type.
    #[must_use]
    pub const fn resolved_type(&self) -> &ResolvedType {
        &self.resolved_type
    }
    /// Returns the final closed default, when present.
    #[must_use]
    pub const fn default_value(&self) -> Option<&LogicalValue> {
        self.default_value.as_ref()
    }
}

/// One immutable nominal type in a captured vocabulary contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyTypeContract {
    /// Exact qualified type identity.
    identity: VocabularyTypeIdentity,
    /// Canonical field contracts.
    fields: Vec<VocabularyFieldContract>,
}

impl VocabularyTypeContract {
    /// Creates one validated vocabulary type contract.
    #[must_use]
    pub fn new(identity: VocabularyTypeIdentity, fields: Vec<VocabularyFieldContract>) -> Self {
        Self { identity, fields }
    }
    /// Returns the exact qualified type identity.
    #[must_use]
    pub const fn identity(&self) -> &VocabularyTypeIdentity {
        &self.identity
    }
    /// Returns canonical field contracts.
    #[must_use]
    pub fn fields(&self) -> &[VocabularyFieldContract] {
        &self.fields
    }
}

/// Complete exact captured vocabulary contract retained in logical IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyContract {
    /// Exact captured and logical identity facts.
    identity: VocabularyIdentity,
    /// Canonical nominal type contracts.
    types: Vec<VocabularyTypeContract>,
}

impl VocabularyContract {
    /// Creates one validated exact vocabulary contract.
    #[must_use]
    pub fn new(identity: VocabularyIdentity, types: Vec<VocabularyTypeContract>) -> Self {
        Self { identity, types }
    }
    /// Returns exact captured and logical identity facts.
    #[must_use]
    pub const fn identity(&self) -> &VocabularyIdentity {
        &self.identity
    }
    /// Returns canonical nominal type contracts.
    #[must_use]
    pub fn types(&self) -> &[VocabularyTypeContract] {
        &self.types
    }
    /// Finds a type without external acquisition.
    #[must_use]
    pub fn type_by_name(&self, name: &str) -> Option<&VocabularyTypeContract> {
        self.types
            .iter()
            .find(|definition| definition.identity().name() == name)
    }

    /// Compares normalized logical vocabulary meaning without captured-byte facts.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        self.identity.identity == other.identity.identity
            && self.identity.version == other.identity.version
            && self.identity.schema_version == other.identity.schema_version
            && self.identity.required_features == other.identity.required_features
            && self.types == other.types
    }
}

/// Immutable validated logical Neutral document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalDocument {
    /// Logical module identity.
    module: LogicalModuleIdentity,
    /// Exported nominal record declarations in canonical name order.
    record_types: Vec<RecordTypeDefinition>,
    /// Optional exact captured vocabulary contract.
    vocabulary: Option<VocabularyContract>,
    /// Exported declarations in deterministic source order.
    declarations: Vec<Declaration>,
}

impl LogicalDocument {
    /// Creates a validated logical document.
    #[must_use]
    pub fn new(module: LogicalModuleIdentity, declarations: Vec<Declaration>) -> Self {
        Self {
            module,
            record_types: Vec::new(),
            vocabulary: None,
            declarations,
        }
    }

    /// Creates a validated logical document containing nominal records and bindings.
    #[must_use]
    pub fn with_record_types(
        module: LogicalModuleIdentity,
        record_types: Vec<RecordTypeDefinition>,
        declarations: Vec<Declaration>,
    ) -> Self {
        Self {
            module,
            record_types,
            vocabulary: None,
            declarations,
        }
    }

    /// Attaches the exact validated captured vocabulary contract.
    #[must_use]
    pub fn with_vocabulary(mut self, vocabulary: VocabularyContract) -> Self {
        self.vocabulary = Some(vocabulary);
        self
    }

    /// Returns the exact captured vocabulary contract, when used.
    #[must_use]
    pub const fn vocabulary(&self) -> Option<&VocabularyContract> {
        self.vocabulary.as_ref()
    }

    /// Returns the logical module identity.
    #[must_use]
    pub const fn module(&self) -> &LogicalModuleIdentity {
        &self.module
    }

    /// Returns exported nominal records in canonical name order.
    #[must_use]
    pub fn record_types(&self) -> &[RecordTypeDefinition] {
        &self.record_types
    }

    /// Finds one exported nominal record by its validated name.
    #[must_use]
    pub fn record_type_by_name(&self, name: &str) -> Option<&RecordTypeDefinition> {
        self.record_types
            .iter()
            .find(|record| record.name() == name)
    }

    /// Returns exported declarations in deterministic order.
    #[must_use]
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Compares the complete logical payload under one bijective element-ID mapping.
    ///
    /// Companion source maps, provenance, derivation facts, and graph-local ID
    /// spellings are intentionally outside this comparison. Both documents must
    /// still contain unique node IDs and references to declared binding nodes.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        let Some(mapping) = ElementIdMapping::between(self, other) else {
            return false;
        };
        self.module == other.module
            && match (&self.vocabulary, &other.vocabulary) {
                (Some(left), Some(right)) => left.logically_equivalent(right),
                (None, None) => true,
                _ => false,
            }
            && logical_record_types_equal(self, other)
            && logical_declarations_equal(self, other, &mapping)
    }
}

/// One temporary bijection between graph-local IDs in two logical documents.
struct ElementIdMapping {
    /// Left-to-right graph-node mapping.
    forward: BTreeMap<ElementId, ElementId>,
    /// Right-to-left graph-node mapping proving injectivity.
    reverse: BTreeMap<ElementId, ElementId>,
}

impl ElementIdMapping {
    /// Builds one complete mapping by matching durable module-symbol identities.
    fn between(left: &LogicalDocument, right: &LogicalDocument) -> Option<Self> {
        if left.record_types.len() != right.record_types.len()
            || left.declarations.len() != right.declarations.len()
        {
            return None;
        }
        let mut mapping = Self {
            forward: BTreeMap::new(),
            reverse: BTreeMap::new(),
        };
        let right_records = unique_records_by_symbol(right)?;
        for record in &left.record_types {
            mapping.insert(
                record.element_id(),
                right_records.get(record.symbol_identity())?.element_id(),
            )?;
        }
        let right_declarations = unique_declarations_by_symbol(right)?;
        for declaration in &left.declarations {
            mapping.insert(
                declaration.element_id(),
                right_declarations
                    .get(declaration.symbol_identity())?
                    .element_id(),
            )?;
        }
        (mapping.forward.len() == left.record_types.len() + left.declarations.len())
            .then_some(mapping)
    }

    /// Adds one pair only when both graph labels remain one-to-one.
    fn insert(&mut self, left: ElementId, right: ElementId) -> Option<()> {
        if self.forward.insert(left, right).is_some() || self.reverse.insert(right, left).is_some()
        {
            return None;
        }
        Some(())
    }

    /// Maps one left document label into the right document.
    fn target(&self, source: ElementId) -> Option<ElementId> {
        self.forward.get(&source).copied()
    }
}

/// Indexes record declarations by durable identity while rejecting duplicates.
fn unique_records_by_symbol(
    document: &LogicalDocument,
) -> Option<BTreeMap<&ModuleSymbolIdentity, &RecordTypeDefinition>> {
    let mut records = BTreeMap::new();
    for record in &document.record_types {
        if records.insert(record.symbol_identity(), record).is_some() {
            return None;
        }
    }
    Some(records)
}

/// Indexes binding declarations by durable identity while rejecting duplicates.
fn unique_declarations_by_symbol(
    document: &LogicalDocument,
) -> Option<BTreeMap<&ModuleSymbolIdentity, &Declaration>> {
    let mut declarations = BTreeMap::new();
    for declaration in &document.declarations {
        if declarations
            .insert(declaration.symbol_identity(), declaration)
            .is_some()
        {
            return None;
        }
    }
    Some(declarations)
}

/// Compares nominal record payloads without graph-local IDs or vector order.
fn logical_record_types_equal(left: &LogicalDocument, right: &LogicalDocument) -> bool {
    let (Some(left_records), Some(right_records)) = (
        unique_records_by_symbol(left),
        unique_records_by_symbol(right),
    ) else {
        return false;
    };
    left_records.iter().all(|(identity, left_record)| {
        right_records.get(identity).is_some_and(|right_record| {
            left_record.fingerprint == right_record.fingerprint
                && left_record.nominal_identity == right_record.nominal_identity
                && left_record.fields == right_record.fields
        })
    })
}

/// Compares binding payloads and every nested identity edge under one mapping.
fn logical_declarations_equal(
    left: &LogicalDocument,
    right: &LogicalDocument,
    mapping: &ElementIdMapping,
) -> bool {
    let (Some(left_declarations), Some(right_declarations)) = (
        unique_declarations_by_symbol(left),
        unique_declarations_by_symbol(right),
    ) else {
        return false;
    };
    left_declarations
        .iter()
        .all(|(identity, left_declaration)| {
            right_declarations
                .get(identity)
                .is_some_and(|right_declaration| {
                    left_declaration.fingerprint == right_declaration.fingerprint
                        && left_declaration.name == right_declaration.name
                        && left_declaration.resolved_type == right_declaration.resolved_type
                        && logical_values_equal(
                            &left_declaration.value,
                            &right_declaration.value,
                            mapping,
                            &left_declarations,
                            &right_declarations,
                        )
                })
        })
}

/// Compares recursively nested values while translating identity-edge targets.
fn logical_values_equal(
    left: &LogicalValue,
    right: &LogicalValue,
    mapping: &ElementIdMapping,
    left_declarations: &BTreeMap<&ModuleSymbolIdentity, &Declaration>,
    right_declarations: &BTreeMap<&ModuleSymbolIdentity, &Declaration>,
) -> bool {
    match (left, right) {
        (LogicalValue::Number(left), LogicalValue::Number(right)) => left == right,
        (LogicalValue::String(left), LogicalValue::String(right)) => left == right,
        (LogicalValue::Boolean(left), LogicalValue::Boolean(right)) => left == right,
        (LogicalValue::Null, LogicalValue::Null) => true,
        (LogicalValue::Record(left), LogicalValue::Record(right)) => {
            left.nominal_type == right.nominal_type
                && left.fields.len() == right.fields.len()
                && left
                    .fields
                    .iter()
                    .zip(&right.fields)
                    .all(|(left_field, right_field)| {
                        left_field.name == right_field.name
                            && logical_values_equal(
                                &left_field.value,
                                &right_field.value,
                                mapping,
                                left_declarations,
                                right_declarations,
                            )
                    })
        }
        (LogicalValue::VocabularyRecord(left), LogicalValue::VocabularyRecord(right)) => {
            left.nominal_type == right.nominal_type
                && left.fields.len() == right.fields.len()
                && left
                    .fields
                    .iter()
                    .zip(&right.fields)
                    .all(|(left_field, right_field)| {
                        left_field.name == right_field.name
                            && logical_values_equal(
                                &left_field.value,
                                &right_field.value,
                                mapping,
                                left_declarations,
                                right_declarations,
                            )
                    })
        }
        (LogicalValue::List(left), LogicalValue::List(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left_item, right_item)| {
                    logical_values_equal(
                        left_item,
                        right_item,
                        mapping,
                        left_declarations,
                        right_declarations,
                    )
                })
        }
        (LogicalValue::Reference(left), LogicalValue::Reference(right)) => {
            left_declarations
                .get(left.target_symbol_identity())
                .is_some_and(|target| target.element_id() == left.target_element_id())
                && right_declarations
                    .get(right.target_symbol_identity())
                    .is_some_and(|target| target.element_id() == right.target_element_id())
                && mapping.target(left.target_element_id()) == Some(right.target_element_id())
                && left.target_symbol_identity() == right.target_symbol_identity()
                && left.target_type() == right.target_type()
        }
        _ => false,
    }
}

/// Source-map entry for one declaration and its minimal child components.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceMapEntry {
    /// Graph-local declaration identifier.
    element_id: ElementId,
    /// Complete declaration source span.
    declaration_span: ByteSpan,
    /// Explicit type source span.
    type_span: ByteSpan,
    /// Declaration-name source span.
    name_span: ByteSpan,
    /// Explicit value source span.
    value_span: ByteSpan,
}

impl SourceMapEntry {
    /// Creates source accounting for one root declaration.
    #[must_use]
    pub const fn new(
        element_id: ElementId,
        declaration_span: ByteSpan,
        type_span: ByteSpan,
        name_span: ByteSpan,
        value_span: ByteSpan,
    ) -> Self {
        Self {
            element_id,
            declaration_span,
            type_span,
            name_span,
            value_span,
        }
    }

    /// Returns the declaration element identifier.
    #[must_use]
    pub const fn element_id(self) -> ElementId {
        self.element_id
    }

    /// Returns the complete declaration span.
    #[must_use]
    pub const fn declaration_span(self) -> ByteSpan {
        self.declaration_span
    }

    /// Returns the explicit type span.
    #[must_use]
    pub const fn type_span(self) -> ByteSpan {
        self.type_span
    }

    /// Returns the declaration-name span.
    #[must_use]
    pub const fn name_span(self) -> ByteSpan {
        self.name_span
    }

    /// Returns the explicit value span.
    #[must_use]
    pub const fn value_span(self) -> ByteSpan {
        self.value_span
    }
}

/// Immutable original-byte source map for a validated document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMap {
    /// Exact captured source identity.
    source_digest: SourceContentDigest,
    /// Exact captured source byte length.
    source_byte_length: u64,
    /// Complete module-header source span.
    module_span: ByteSpan,
    /// Declaration mappings.
    entries: Vec<SourceMapEntry>,
}

impl SourceMap {
    /// Creates an immutable source map from validated compiler accounting.
    #[must_use]
    pub fn new(
        source_digest: SourceContentDigest,
        source_byte_length: u64,
        module_span: ByteSpan,
        entries: Vec<SourceMapEntry>,
    ) -> Self {
        Self {
            source_digest,
            source_byte_length,
            module_span,
            entries,
        }
    }

    /// Returns the exact captured source identity.
    #[must_use]
    pub const fn source_digest(&self) -> SourceContentDigest {
        self.source_digest
    }

    /// Returns the exact captured source byte length.
    #[must_use]
    pub const fn source_byte_length(&self) -> u64 {
        self.source_byte_length
    }

    /// Returns the complete module-header source span.
    #[must_use]
    pub const fn module_span(&self) -> ByteSpan {
        self.module_span
    }

    /// Returns deterministic declaration source mappings.
    #[must_use]
    pub fn entries(&self) -> &[SourceMapEntry] {
        &self.entries
    }

    /// Finds source accounting for one graph-local element.
    #[must_use]
    pub fn entry(&self, element_id: ElementId) -> Option<&SourceMapEntry> {
        self.entries
            .iter()
            .find(|entry| entry.element_id == element_id)
    }
}

/// Why a final logical value exists in the active document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueOrigin {
    /// Value was written explicitly in captured source.
    ExplicitSource,
    /// A binding's final logical value was reused from another immutable binding.
    OrdinaryReuse,
    /// A typed identity edge was resolved from explicit `ref(name)` source.
    IdentityReference,
    /// A contextual record field was written explicitly.
    ExplicitRecordField,
    /// An omitted contextual field was materialized from a user-record default.
    UserRecordDefault,
    /// An omitted vocabulary-owned field was materialized from its captured contract.
    VocabularyDefault,
}

impl ValueOrigin {
    /// Returns the stable vocabulary spelling used by generic consumers.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitSource => "explicit-source",
            Self::OrdinaryReuse => "ordinary-reuse",
            Self::IdentityReference => "identity-reference",
            Self::ExplicitRecordField => "explicit-record-field",
            Self::UserRecordDefault => "user-record-default",
            Self::VocabularyDefault => "vocabulary-default",
        }
    }
}

/// Logical normalization applied while lowering one value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Normalization {
    /// Exact source number was normalized without host floating point.
    ExactNumberCanonicalization,
    /// String escapes were decoded to their exact Unicode scalar sequence.
    StringEscapeDecoding,
    /// A Boolean token was lowered without value transformation.
    BooleanIdentity,
    /// An explicit null token was lowered without structural omission.
    NullIdentity,
    /// Explicit record fields were matched to one nominal contextual schema.
    RecordContextualization,
    /// Ordered list items were checked against one contextual element type.
    ListContextualization,
    /// An ordinary immutable binding name was replaced by its final logical value.
    ImmutableValueReuse,
    /// A `ref(name)` target was resolved to a typed document-local identity edge.
    IdentityReferenceResolution,
}

impl Normalization {
    /// Returns the stable external vocabulary spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExactNumberCanonicalization => "exact-number",
            Self::StringEscapeDecoding => "string-escape",
            Self::BooleanIdentity => "boolean-identity",
            Self::NullIdentity => "null-identity",
            Self::RecordContextualization => "record-context",
            Self::ListContextualization => "list-context",
            Self::ImmutableValueReuse => "immutable-reuse",
            Self::IdentityReferenceResolution => "identity-reference",
        }
    }
}

/// Provenance record for one minimal binding value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvenanceRecord {
    /// Graph-local element whose value is explained.
    element_id: ElementId,
    /// Why the value exists.
    origin: ValueOrigin,
    /// Logical normalization applied to the source value.
    normalization: Normalization,
}

impl ProvenanceRecord {
    /// Creates provenance for an explicit normalized numeric binding.
    #[must_use]
    pub const fn new(
        element_id: ElementId,
        origin: ValueOrigin,
        normalization: Normalization,
    ) -> Self {
        Self {
            element_id,
            origin,
            normalization,
        }
    }

    /// Returns the owning graph-local element.
    #[must_use]
    pub const fn element_id(self) -> ElementId {
        self.element_id
    }

    /// Returns why the value exists.
    #[must_use]
    pub const fn origin(self) -> ValueOrigin {
        self.origin
    }

    /// Returns the logical normalization applied to the value.
    #[must_use]
    pub const fn normalization(self) -> Normalization {
        self.normalization
    }
}

/// Provenance for one final field inside a binding's contextual record value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldProvenanceRecord {
    /// Graph-local binding element that owns the record value.
    element_id: ElementId,
    /// Canonical field path from the binding value root.
    field_path: Vec<String>,
    /// Whether the field was explicit or supplied by a user default.
    origin: ValueOrigin,
}

/// Provenance for one ordinary immutable-value reuse edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReuseProvenanceRecord {
    /// Graph-local binding element that owns the reused value occurrence.
    element_id: ElementId,
    /// Canonical record-field/list-index path from the binding value root.
    value_path: Vec<String>,
    /// Graph-local binding element whose final logical value was reused.
    source_element_id: ElementId,
}

impl ReuseProvenanceRecord {
    /// Creates one validated ordinary immutable-value reuse edge.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        value_path: Vec<String>,
        source_element_id: ElementId,
    ) -> Self {
        Self {
            element_id,
            value_path,
            source_element_id,
        }
    }

    /// Returns the binding that owns the reused value occurrence.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the canonical path from the owning binding value root.
    #[must_use]
    pub fn value_path(&self) -> &[String] {
        &self.value_path
    }

    /// Returns the binding whose final logical value was reused.
    #[must_use]
    pub const fn source_element_id(&self) -> ElementId {
        self.source_element_id
    }
}

/// Provenance for one typed identity-reference edge occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceProvenanceRecord {
    /// Graph-local binding element that owns the reference occurrence.
    element_id: ElementId,
    /// Canonical record-field/list-index path from the binding value root.
    value_path: Vec<String>,
    /// Authoritative graph-local target binding identifier.
    target_element_id: ElementId,
}

impl ReferenceProvenanceRecord {
    /// Creates one validated typed identity-reference provenance edge.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        value_path: Vec<String>,
        target_element_id: ElementId,
    ) -> Self {
        Self {
            element_id,
            value_path,
            target_element_id,
        }
    }

    /// Returns the binding that owns the reference occurrence.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the canonical path from the owning binding value root.
    #[must_use]
    pub fn value_path(&self) -> &[String] {
        &self.value_path
    }

    /// Returns the authoritative document-local target binding identifier.
    #[must_use]
    pub const fn target_element_id(&self) -> ElementId {
        self.target_element_id
    }
}

impl FieldProvenanceRecord {
    /// Creates one validated record-field provenance entry.
    #[must_use]
    pub fn new(element_id: ElementId, field_path: Vec<String>, origin: ValueOrigin) -> Self {
        Self {
            element_id,
            field_path,
            origin,
        }
    }

    /// Returns the owning binding element.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the canonical field path from the binding root.
    #[must_use]
    pub fn field_path(&self) -> &[String] {
        &self.field_path
    }

    /// Returns why this final field value exists.
    #[must_use]
    pub const fn origin(&self) -> ValueOrigin {
        self.origin
    }
}

/// Deterministic resource facts recorded for successful compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceFacts {
    /// Exact captured source bytes.
    source_bytes: u64,
    /// Number of authoritative declarations.
    declarations: u64,
    /// Number of retained diagnostics.
    diagnostics: u64,
    /// Decoded UTF-8 bytes retained by string values.
    decoded_string_bytes: u64,
}

impl ResourceFacts {
    /// Creates successful-compilation resource accounting.
    #[must_use]
    pub const fn new(
        source_bytes: u64,
        declarations: u64,
        diagnostics: u64,
        decoded_string_bytes: u64,
    ) -> Self {
        Self {
            source_bytes,
            declarations,
            diagnostics,
            decoded_string_bytes,
        }
    }

    /// Returns the exact captured source byte count.
    #[must_use]
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }

    /// Returns the authoritative declaration count.
    #[must_use]
    pub const fn declarations(self) -> u64 {
        self.declarations
    }

    /// Returns the retained diagnostic count.
    #[must_use]
    pub const fn diagnostics(self) -> u64 {
        self.diagnostics
    }

    /// Returns decoded UTF-8 bytes retained by string values.
    #[must_use]
    pub const fn decoded_string_bytes(self) -> u64 {
        self.decoded_string_bytes
    }
}

/// Meaning-affecting derivation inputs for captured compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeaningPartition {
    /// Exact captured source content identity.
    source_digest: SourceContentDigest,
}

impl MeaningPartition {
    /// Returns the exact source content identity.
    #[must_use]
    pub const fn source_digest(self) -> SourceContentDigest {
        self.source_digest
    }
}

/// Acceptance and resource inputs for captured compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptancePartition {
    /// Maximum accepted captured source bytes.
    source_bytes: u64,
    /// Maximum retained diagnostics.
    diagnostics: u32,
    /// Maximum decoded UTF-8 bytes in one string scalar.
    string_bytes: u64,
    /// Maximum significant decimal digits in one exact number.
    numeric_digits: u64,
    /// Maximum absolute decimal scale in one exact number.
    numeric_scale: u64,
    /// Maximum root declarations.
    declarations: u64,
    /// Maximum record fields per declaration or value.
    record_fields: u64,
    /// Maximum contextual-record nesting depth.
    nesting_depth: u64,
    /// Maximum list items per value.
    list_items: u64,
    /// Maximum recursively traversed value nodes.
    traversal_nodes: u64,
}

impl AcceptancePartition {
    /// Captures the complete resource-limit partition from core limits.
    #[must_use]
    pub const fn from_limits(limits: StructuralLimits) -> Self {
        Self {
            source_bytes: limits.source_bytes(),
            diagnostics: limits.diagnostics(),
            string_bytes: limits.string_bytes(),
            numeric_digits: limits.numeric_digits(),
            numeric_scale: limits.numeric_scale(),
            declarations: limits.declarations(),
            record_fields: limits.record_fields(),
            nesting_depth: limits.nesting_depth(),
            list_items: limits.list_items(),
            traversal_nodes: limits.traversal_nodes(),
        }
    }

    /// Returns the captured source-byte limit.
    #[must_use]
    pub const fn source_byte_limit(self) -> u64 {
        self.source_bytes
    }

    /// Returns the captured diagnostic limit.
    #[must_use]
    pub const fn diagnostic_limit(self) -> u32 {
        self.diagnostics
    }

    /// Returns the decoded string-byte limit.
    #[must_use]
    pub const fn string_byte_limit(self) -> u64 {
        self.string_bytes
    }

    /// Returns the exact-number significant-digit limit.
    #[must_use]
    pub const fn numeric_digit_limit(self) -> u64 {
        self.numeric_digits
    }

    /// Returns the exact-number absolute-scale limit.
    #[must_use]
    pub const fn numeric_scale_limit(self) -> u64 {
        self.numeric_scale
    }

    /// Returns the root declaration-count limit.
    #[must_use]
    pub const fn declaration_limit(self) -> u64 {
        self.declarations
    }

    /// Returns the per-record field-count limit.
    #[must_use]
    pub const fn record_field_limit(self) -> u64 {
        self.record_fields
    }

    /// Returns the contextual-record nesting-depth limit.
    #[must_use]
    pub const fn nesting_depth_limit(self) -> u64 {
        self.nesting_depth
    }

    /// Returns the per-list item-count limit.
    #[must_use]
    pub const fn list_item_limit(self) -> u64 {
        self.list_items
    }

    /// Returns the recursive value traversal-node limit.
    #[must_use]
    pub const fn traversal_node_limit(self) -> u64 {
        self.traversal_nodes
    }
}

/// Diagnostic/output-policy inputs for captured compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticPartition {
    /// Whether safe bounded diagnostics are enabled.
    safe_bounded_output: bool,
}

impl DiagnosticPartition {
    /// Returns whether diagnostics use the safe bounded output contract.
    #[must_use]
    pub const fn safe_bounded_output(self) -> bool {
        self.safe_bounded_output
    }
}

/// Immutable partitioned derivation manifest for successful compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationManifest {
    /// Frozen language behavior version.
    language_behavior_version: String,
    /// Meaning-affecting inputs.
    meaning: MeaningPartition,
    /// Acceptance and resource inputs.
    acceptance: AcceptancePartition,
    /// Diagnostic and output-policy inputs.
    diagnostics: DiagnosticPartition,
    /// Observed deterministic resource facts.
    resource_facts: ResourceFacts,
    /// Exact vocabulary identity facts when the source uses one capture.
    vocabulary: Option<VocabularyIdentity>,
}

impl DerivationManifest {
    /// Creates the minimal partitioned derivation manifest.
    #[must_use]
    pub fn new(
        language_behavior_version: impl Into<String>,
        source_digest: SourceContentDigest,
        acceptance: AcceptancePartition,
        resource_facts: ResourceFacts,
    ) -> Self {
        Self {
            language_behavior_version: language_behavior_version.into(),
            meaning: MeaningPartition { source_digest },
            acceptance,
            diagnostics: DiagnosticPartition {
                safe_bounded_output: true,
            },
            resource_facts,
            vocabulary: None,
        }
    }

    /// Attaches exact meaning-affecting captured vocabulary facts.
    #[must_use]
    pub fn with_vocabulary(mut self, vocabulary: VocabularyIdentity) -> Self {
        self.vocabulary = Some(vocabulary);
        self
    }

    /// Returns exact captured vocabulary derivation facts, when present.
    #[must_use]
    pub const fn vocabulary(&self) -> Option<&VocabularyIdentity> {
        self.vocabulary.as_ref()
    }

    /// Returns the language behavior version.
    #[must_use]
    pub fn language_behavior_version(&self) -> &str {
        &self.language_behavior_version
    }

    /// Returns the logical IR schema version.
    #[must_use]
    pub const fn logical_ir_schema_version(&self) -> &'static str {
        LOGICAL_IR_SCHEMA_VERSION
    }

    /// Returns the source-map schema version.
    #[must_use]
    pub const fn source_map_version(&self) -> &'static str {
        SOURCE_MAP_VERSION
    }

    /// Returns the provenance schema version.
    #[must_use]
    pub const fn provenance_version(&self) -> &'static str {
        PROVENANCE_VERSION
    }

    /// Returns the meaning-affecting partition.
    #[must_use]
    pub const fn meaning(&self) -> MeaningPartition {
        self.meaning
    }

    /// Returns the acceptance/resource partition.
    #[must_use]
    pub const fn acceptance(&self) -> AcceptancePartition {
        self.acceptance
    }

    /// Returns the diagnostic/output partition.
    #[must_use]
    pub const fn diagnostics(&self) -> DiagnosticPartition {
        self.diagnostics
    }

    /// Returns observed deterministic resource facts.
    #[must_use]
    pub const fn resource_facts(&self) -> ResourceFacts {
        self.resource_facts
    }
}

/// Complete immutable successful compiler output before optional encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationArtifacts {
    /// Authoritative logical document.
    logical_document: LogicalDocument,
    /// Original-byte source map.
    source_map: SourceMap,
    /// Value provenance records.
    provenance: Vec<ProvenanceRecord>,
    /// Field-level explicit/default provenance records.
    field_provenance: Vec<FieldProvenanceRecord>,
    /// Ordinary immutable-value reuse edges.
    reuse_provenance: Vec<ReuseProvenanceRecord>,
    /// Typed identity-reference provenance edges.
    reference_provenance: Vec<ReferenceProvenanceRecord>,
    /// Partitioned derivation manifest.
    derivation: DerivationManifest,
}

impl CompilationArtifacts {
    /// Creates already-validated in-process compilation artifacts.
    #[must_use]
    pub fn new(
        logical_document: LogicalDocument,
        source_map: SourceMap,
        provenance: Vec<ProvenanceRecord>,
        derivation: DerivationManifest,
    ) -> Self {
        Self {
            logical_document,
            source_map,
            provenance,
            field_provenance: Vec::new(),
            reuse_provenance: Vec::new(),
            reference_provenance: Vec::new(),
            derivation,
        }
    }

    /// Compares only logical payloads under graph-local ID alpha-renaming.
    ///
    /// This deliberately excludes source maps, provenance, derivation facts,
    /// and other companion or envelope records. Exact artifact equality remains
    /// available through `PartialEq` when those records must also match.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        self.logical_document
            .logically_equivalent(&other.logical_document)
    }

    /// Attaches validated field-level provenance to successful artifacts.
    #[must_use]
    pub fn with_field_provenance(mut self, field_provenance: Vec<FieldProvenanceRecord>) -> Self {
        self.field_provenance = field_provenance;
        self
    }

    /// Attaches validated ordinary immutable-value reuse provenance.
    #[must_use]
    pub fn with_reuse_provenance(mut self, reuse_provenance: Vec<ReuseProvenanceRecord>) -> Self {
        self.reuse_provenance = reuse_provenance;
        self
    }

    /// Attaches validated typed identity-reference provenance.
    #[must_use]
    pub fn with_reference_provenance(
        mut self,
        reference_provenance: Vec<ReferenceProvenanceRecord>,
    ) -> Self {
        self.reference_provenance = reference_provenance;
        self
    }

    /// Returns the authoritative logical document.
    #[must_use]
    pub const fn logical_document(&self) -> &LogicalDocument {
        &self.logical_document
    }

    /// Returns the exact original-byte source map.
    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Returns deterministic value provenance records.
    #[must_use]
    pub fn provenance(&self) -> &[ProvenanceRecord] {
        &self.provenance
    }

    /// Returns deterministic explicit/default record-field provenance.
    #[must_use]
    pub fn field_provenance(&self) -> &[FieldProvenanceRecord] {
        &self.field_provenance
    }

    /// Returns deterministic ordinary immutable-value reuse edges.
    #[must_use]
    pub fn reuse_provenance(&self) -> &[ReuseProvenanceRecord] {
        &self.reuse_provenance
    }

    /// Returns deterministic typed identity-reference provenance edges.
    #[must_use]
    pub fn reference_provenance(&self) -> &[ReferenceProvenanceRecord] {
        &self.reference_provenance
    }

    /// Returns the partitioned derivation manifest.
    #[must_use]
    pub const fn derivation(&self) -> &DerivationManifest {
        &self.derivation
    }
}

/// A failure while constructing validated logical IR values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrError {
    /// A source numeric spelling was empty or contained non-digits.
    InvalidExactNumber,
    /// A source numeric spelling exceeded an explicit deterministic bound.
    ExactNumberLimitExceeded,
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
