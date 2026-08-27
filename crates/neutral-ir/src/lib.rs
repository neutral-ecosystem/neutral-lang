// SPDX-License-Identifier: Apache-2.0

//! Public logical representation contracts for Neutral artifacts.
//!
//! This crate owns the logical IR, source maps, provenance, and derivation
//! records. It must not acquire source input, expose compiler-private models, or
//! perform host I/O.

use neutral_core::{ByteSpan, CoreError, SemanticDigest, SourceContentDigest, nht_frame};
use std::fmt;

/// Frozen logical IR schema version for the minimal v0 artifact.
pub const LOGICAL_IR_SCHEMA_VERSION: &str = "0.1.0";
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementId(u64);

impl ElementId {
    /// Creates a validated-document-local element identifier.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the local integer label for indexing within one document.
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
    /// Normalizes one unsigned digits-only integer without host numeric conversion.
    ///
    /// # Errors
    ///
    /// Returns an error when the spelling is empty or contains non-digits.
    pub fn from_unsigned_integer(spelling: &str) -> Result<Self, IrError> {
        if spelling.is_empty() || !spelling.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(IrError::InvalidExactNumber);
        }
        let coefficient = spelling.trim_start_matches('0');
        let coefficient = if coefficient.is_empty() {
            "0"
        } else {
            coefficient
        };
        Ok(Self {
            negative: false,
            coefficient: coefficient.to_owned(),
            scale: 0,
        })
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

/// Resolved minimal Neutral type identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResolvedType {
    /// Exact Neutral numeric type.
    Num,
    /// Unicode scalar-sequence string type.
    String,
    /// Exact Boolean type.
    Bool,
}

impl fmt::Display for ResolvedType {
    /// Formats the stable source-facing type name.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num => formatter.write_str("num"),
            Self::String => formatter.write_str("string"),
            Self::Bool => formatter.write_str("bool"),
        }
    }
}

/// Immutable logical value in the minimal IR slice.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LogicalValue {
    /// Exact normalized Neutral number.
    Number(ExactNumber),
    /// Exact decoded Unicode scalar sequence.
    String(String),
    /// Exact Boolean value.
    Boolean(bool),
}

impl fmt::Display for LogicalValue {
    /// Formats a deterministic reader-facing logical value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(formatter),
            Self::String(value) => format_safe_string(value, formatter),
            Self::Boolean(value) => value.fmt(formatter),
        }
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
    /// Computes the v1 binding fingerprint over resolved type and logical value.
    ///
    /// # Errors
    ///
    /// Returns an error only when NHT framing exceeds its fixed widths.
    pub fn for_binding(
        resolved_type: ResolvedType,
        value: &LogicalValue,
    ) -> Result<Self, CoreError> {
        let mut payload = Vec::new();
        payload.extend(nht_frame("declaration-kind", b"binding")?);
        payload.extend(nht_frame(
            "resolved-type",
            resolved_type.to_string().as_bytes(),
        )?);
        let definition = match value {
            LogicalValue::Number(number) => number.nht_payload()?,
            LogicalValue::String(value) => nht_frame("string", value.as_bytes())?,
            LogicalValue::Boolean(value) => nht_frame("boolean", &[u8::from(*value)])?,
        };
        payload.extend(nht_frame("logical-definition", &definition)?);
        SemanticDigest::from_nht("neutral/declaration-fingerprint/v1", &payload).map(Self)
    }

    /// Returns the underlying domain-separated semantic digest.
    #[must_use]
    pub const fn digest(self) -> SemanticDigest {
        self.0
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
    pub const fn resolved_type(&self) -> ResolvedType {
        self.resolved_type
    }

    /// Returns the final immutable logical value.
    #[must_use]
    pub const fn value(&self) -> &LogicalValue {
        &self.value
    }
}

/// Immutable validated logical Neutral document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalDocument {
    /// Logical module identity.
    module: LogicalModuleIdentity,
    /// Exported declarations in deterministic source order.
    declarations: Vec<Declaration>,
}

impl LogicalDocument {
    /// Creates a validated logical document.
    #[must_use]
    pub fn new(module: LogicalModuleIdentity, declarations: Vec<Declaration>) -> Self {
        Self {
            module,
            declarations,
        }
    }

    /// Returns the logical module identity.
    #[must_use]
    pub const fn module(&self) -> &LogicalModuleIdentity {
        &self.module
    }

    /// Returns exported declarations in deterministic order.
    #[must_use]
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Compares logical meaning while ignoring graph-local element labels.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        self.module == other.module
            && self.declarations.len() == other.declarations.len()
            && self
                .declarations
                .iter()
                .zip(&other.declarations)
                .all(|(left, right)| {
                    left.symbol_identity == right.symbol_identity
                        && left.fingerprint == right.fingerprint
                        && left.name == right.name
                        && left.resolved_type == right.resolved_type
                        && left.value == right.value
                })
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
    /// Creates source accounting for one minimal declaration.
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

/// Why a final logical value exists in the minimal document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueOrigin {
    /// Value was written explicitly in captured source.
    ExplicitSource,
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

/// Meaning-affecting derivation inputs for the minimal compilation.
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

/// Acceptance and resource inputs for the minimal compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptancePartition {
    /// Maximum accepted captured source bytes.
    source_bytes: u64,
    /// Maximum retained diagnostics.
    diagnostics: u32,
    /// Maximum decoded UTF-8 bytes in one string scalar.
    string_bytes: u64,
}

impl AcceptancePartition {
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
}

/// Diagnostic/output-policy inputs for the minimal compilation.
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
}

impl DerivationManifest {
    /// Creates the minimal partitioned derivation manifest.
    #[must_use]
    pub fn new(
        language_behavior_version: impl Into<String>,
        source_digest: SourceContentDigest,
        source_byte_limit: u64,
        diagnostic_limit: u32,
        string_byte_limit: u64,
        resource_facts: ResourceFacts,
    ) -> Self {
        Self {
            language_behavior_version: language_behavior_version.into(),
            meaning: MeaningPartition { source_digest },
            acceptance: AcceptancePartition {
                source_bytes: source_byte_limit,
                diagnostics: diagnostic_limit,
                string_bytes: string_byte_limit,
            },
            diagnostics: DiagnosticPartition {
                safe_bounded_output: true,
            },
            resource_facts,
        }
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
            derivation,
        }
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
}

#[cfg(test)]
/// Unit tests for public logical IR identity and equality contracts.
mod tests {
    use super::{DeclarationFingerprint, ExactNumber, LogicalValue, ResolvedType};

    #[test]
    /// Verifies minimal integer normalization does not use host numeric types.
    fn exact_integer_normalization_removes_only_leading_zeroes() {
        let number = ExactNumber::from_unsigned_integer("00042")
            .expect("digits-only integer should normalize");
        assert_eq!(number.coefficient(), "42");
        assert_eq!(number.to_string(), "42/1");
    }

    #[test]
    /// Verifies declaration names do not enter logical definition fingerprints.
    fn binding_fingerprint_depends_on_type_and_logical_value() {
        let value = LogicalValue::Number(
            ExactNumber::from_unsigned_integer("42").expect("integer should normalize"),
        );
        let first = DeclarationFingerprint::for_binding(ResolvedType::Num, &value)
            .expect("fingerprint should succeed");
        let second = DeclarationFingerprint::for_binding(ResolvedType::Num, &value)
            .expect("fingerprint should be deterministic");
        assert_eq!(first, second);
    }

    #[test]
    /// Verifies logical strings render every decoded control through safe escapes.
    fn logical_string_rendering_escapes_controls_and_delimiters() {
        let value = LogicalValue::String("quote:\" slash:\\ line:\n nul:\0".to_owned());
        assert_eq!(
            value.to_string(),
            "\"quote:\\\" slash:\\\\ line:\\n nul:\\0\""
        );
    }

    #[test]
    /// Verifies scalar types and Boolean values enter definition fingerprints.
    fn scalar_fingerprints_distinguish_type_and_boolean_value() {
        let truth =
            DeclarationFingerprint::for_binding(ResolvedType::Bool, &LogicalValue::Boolean(true))
                .expect("Boolean fingerprint should succeed");
        let falsehood =
            DeclarationFingerprint::for_binding(ResolvedType::Bool, &LogicalValue::Boolean(false))
                .expect("Boolean fingerprint should succeed");
        let text = DeclarationFingerprint::for_binding(
            ResolvedType::String,
            &LogicalValue::String("true".to_owned()),
        )
        .expect("string fingerprint should succeed");
        assert_ne!(truth, falsehood);
        assert_ne!(truth, text);
    }
}
