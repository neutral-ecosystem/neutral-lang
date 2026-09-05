// SPDX-License-Identifier: Apache-2.0

//! Frozen framing, capability, and schema spellings for the v0 encoder.

/// Frozen external encoding identifier.
pub const ENCODING: &str = "NIR-CBOR/0.1";
/// Frozen media type for encoded artifacts.
pub const MEDIA_TYPE: &str = "application/vnd.neutral.ir+cbor;version=0.1";
/// Fixed outer-frame magic bytes.
pub const MAGIC: [u8; 8] = *b"NEUIR\r\n\x1a";
/// Fixed framing revision.
pub const FRAMING_REVISION: u16 = 1;
/// Fixed header size.
pub const HEADER_BYTES: usize = 48;
/// Fixed directory-entry size.
pub const DIRECTORY_ENTRY_BYTES: usize = 24;
/// Fixed section count.
pub const SECTION_COUNT: usize = 5;
/// Fixed section schema revision.
pub const SECTION_SCHEMA_REVISION: u16 = 1;
/// Maximum complete artifact size.
pub const MAXIMUM_ARTIFACT_BYTES: usize = 67_108_864;
/// Maximum encoded section size.
pub const MAXIMUM_SECTION_BYTES: usize = 67_108_864;
/// Maximum nested type/value layers.
pub const MAXIMUM_NESTING_DEPTH: usize = 128;
/// Maximum entries in one array or map.
pub const MAXIMUM_CONTAINER_ITEMS: usize = 1_000_000;
/// Maximum UTF-8 bytes in one text item.
pub const MAXIMUM_TEXT_BYTES: usize = 16_777_216;
/// Maximum bytes in one byte-string item.
pub const MAXIMUM_BYTE_STRING_BYTES: usize = 16_777_216;
/// Maximum exact-number coefficient digits.
pub const MAXIMUM_EXACT_NUMBER_DIGITS: usize = 1_000_000;
/// Maximum recursively traversed logical nodes.
pub const MAXIMUM_TRAVERSAL_NODES: usize = 1_000_000;

/// Capability names in their required bit order.
pub const CAPABILITY_NAMES: [&str; 8] = [
    "exact-decimal",
    "nullable",
    "nominal-record",
    "ordered-list",
    "identity-reference",
    "captured-vocabulary",
    "ordinary-reuse-provenance",
    "default-provenance",
];

/// Schema field and discriminator spellings.
pub mod key {
    /// Acceptance partition.
    pub const ACCEPTANCE: &str = "acceptance";
    /// Build identifier.
    pub const BUILD: &str = "build";
    /// Boolean/value discriminator.
    pub const KIND: &str = "kind";
    /// Capability list.
    pub const REQUIRED_CAPABILITIES: &str = "required_capabilities";
    /// Coefficient.
    pub const COEFFICIENT: &str = "coefficient";
    /// Content digest.
    pub const CONTENT_DIGEST: &str = "content_digest";
    /// Declarations collection/count.
    pub const DECLARATIONS: &str = "declarations";
    /// Declaration span.
    pub const DECLARATION_SPAN: &str = "declaration_span";
    /// Decoded string bytes.
    pub const DECODED_STRING_BYTES: &str = "decoded_string_bytes";
    /// Optional default value.
    pub const DEFAULT: &str = "default";
    /// Derivation schema version.
    pub const DERIVATION_SCHEMA_VERSION: &str = "derivation_schema_version";
    /// Derivation version envelope key.
    pub const DERIVATION: &str = "derivation";
    /// Diagnostic count.
    pub const DIAGNOSTICS: &str = "diagnostics";
    /// Diagnostic policy.
    pub const DIAGNOSTIC_POLICY: &str = "diagnostic_policy";
    /// Element identifier.
    pub const ELEMENT_ID: &str = "element_id";
    /// Encoding identifier/version.
    pub const ENCODING: &str = "encoding";
    /// Encoding version.
    pub const ENCODING_VERSION: &str = "encoding_version";
    /// End byte offset.
    pub const END: &str = "end";
    /// Field collection.
    pub const FIELDS: &str = "fields";
    /// Field path.
    pub const FIELD_PATH: &str = "field_path";
    /// Fingerprint.
    pub const FINGERPRINT: &str = "fingerprint";
    /// Framing revision.
    pub const FRAMING_REVISION: &str = "framing_revision";
    /// Identity.
    pub const IDENTITY: &str = "identity";
    /// Inner type.
    pub const INNER: &str = "inner";
    /// Integrity records.
    pub const INTEGRITY: &str = "integrity";
    /// Item values.
    pub const ITEMS: &str = "items";
    /// Language behavior version.
    pub const LANGUAGE_BEHAVIOR_VERSION: &str = "language_behavior_version";
    /// Language behavior envelope version.
    pub const LANGUAGE_BEHAVIOR: &str = "language_behavior";
    /// List-item limit.
    pub const LIST_ITEMS: &str = "list_items";
    /// Logical IR schema version.
    pub const LOGICAL_IR_SCHEMA_VERSION: &str = "logical_ir_schema_version";
    /// Logical IR envelope version.
    pub const LOGICAL_IR: &str = "logical_ir";
    /// Meaning partition.
    pub const MEANING: &str = "meaning";
    /// Module identity.
    pub const MODULE: &str = "module";
    /// Module span.
    pub const MODULE_SPAN: &str = "module_span";
    /// Name.
    pub const NAME: &str = "name";
    /// Name span.
    pub const NAME_SPAN: &str = "name_span";
    /// Negative number flag.
    pub const NEGATIVE: &str = "negative";
    /// Nesting depth.
    pub const NESTING_DEPTH: &str = "nesting_depth";
    /// Normalization.
    pub const NORMALIZATION: &str = "normalization";
    /// Numeric digit limit.
    pub const NUMERIC_DIGITS: &str = "numeric_digits";
    /// Numeric scale limit.
    pub const NUMERIC_SCALE: &str = "numeric_scale";
    /// Origin.
    pub const ORIGIN: &str = "origin";
    /// Producer map.
    pub const PRODUCER: &str = "producer";
    /// Provenance version.
    pub const PROVENANCE_VERSION: &str = "provenance_version";
    /// Provenance envelope version.
    pub const PROVENANCE: &str = "provenance";
    /// Record fields limit.
    pub const RECORD_FIELDS: &str = "record_fields";
    /// Record types.
    pub const RECORD_TYPES: &str = "record_types";
    /// Reference provenance.
    pub const REFERENCES: &str = "references";
    /// Required vocabulary features.
    pub const REQUIRED_FEATURES: &str = "required_features";
    /// Resource facts.
    pub const RESOURCE_FACTS: &str = "resource_facts";
    /// Reuse provenance.
    pub const REUSES: &str = "reuses";
    /// Safe bounded output flag.
    pub const SAFE_BOUNDED_OUTPUT: &str = "safe_bounded_output";
    /// Decimal scale.
    pub const SCALE: &str = "scale";
    /// Schema version.
    pub const SCHEMA_VERSION: &str = "schema_version";
    /// Integrity section kind.
    pub const SECTION: &str = "section";
    /// Integrity SHA-256 bytes.
    pub const SHA256: &str = "sha256";
    /// Source byte length.
    pub const SOURCE_BYTE_LENGTH: &str = "source_byte_length";
    /// Source byte count/limit.
    pub const SOURCE_BYTES: &str = "source_bytes";
    /// Source digest.
    pub const SOURCE_DIGEST: &str = "source_digest";
    /// Source element identifier.
    pub const SOURCE_ELEMENT_ID: &str = "source_element_id";
    /// Source-map entries.
    pub const ENTRIES: &str = "entries";
    /// Source-map version.
    pub const SOURCE_MAP_VERSION: &str = "source_map_version";
    /// Source-map envelope version.
    pub const SOURCE_MAP: &str = "source_map";
    /// Start byte offset.
    pub const START: &str = "start";
    /// String byte limit.
    pub const STRING_BYTES: &str = "string_bytes";
    /// Symbol identity.
    pub const SYMBOL: &str = "symbol";
    /// Target element identifier.
    pub const TARGET_ELEMENT_ID: &str = "target_element_id";
    /// Target symbol.
    pub const TARGET_SYMBOL: &str = "target_symbol";
    /// Target type.
    pub const TARGET_TYPE: &str = "target_type";
    /// Traversal-node limit.
    pub const TRAVERSAL_NODES: &str = "traversal_nodes";
    /// Resolved type.
    pub const TYPE: &str = "type";
    /// Type span.
    pub const TYPE_SPAN: &str = "type_span";
    /// Vocabulary types.
    pub const TYPES: &str = "types";
    /// Logical/default value.
    pub const VALUE: &str = "value";
    /// Value path.
    pub const VALUE_PATH: &str = "value_path";
    /// Value span.
    pub const VALUE_SPAN: &str = "value_span";
    /// Value provenance.
    pub const VALUES: &str = "values";
    /// Version.
    pub const VERSION: &str = "version";
    /// Version map.
    pub const VERSIONS: &str = "versions";
    /// Vocabulary contract/identity.
    pub const VOCABULARY: &str = "vocabulary";
    /// Vocabulary schema envelope version.
    pub const VOCABULARY_SCHEMA: &str = "vocabulary_schema";
}

/// Closed sum discriminator spellings.
pub mod kind {
    /// Boolean kind.
    pub const BOOL: &str = "bool";
    /// List kind.
    pub const LIST: &str = "list";
    /// Null kind.
    pub const NULL: &str = "null";
    /// Number kind.
    pub const NUM: &str = "num";
    /// Record kind.
    pub const RECORD: &str = "record";
    /// Reference kind.
    pub const REF: &str = "ref";
    /// String kind.
    pub const STRING: &str = "string";
    /// Nullable kind.
    pub const NULLABLE: &str = "nullable";
    /// Vocabulary record kind.
    pub const VOCABULARY_RECORD: &str = "vocabulary-record";
}
