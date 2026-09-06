// SPDX-License-Identifier: Apache-2.0

//! Strict parsing for the stable host-facing command interface.

use crate::constants;
use neutral_core::StructuralLimits;

/// One supported host operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandKind {
    /// Compile source into an encoded artifact.
    Compile,
    /// Validate source without authoritative output.
    Validate,
    /// Format valid source into canonical source bytes.
    Format,
}

/// Exact optional vocabulary bundle and immutable lock arguments.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VocabularyOptions {
    /// Explicit bundle input path.
    pub bundle: Option<String>,
    /// Expected logical identity.
    pub identity: Option<String>,
    /// Expected release version.
    pub version: Option<String>,
    /// Expected bundle encoding version.
    pub encoding_version: Option<String>,
    /// Expected logical schema version.
    pub schema_version: Option<String>,
    /// Expected exact content digest text.
    pub digest: Option<String>,
    /// Required structural features.
    pub features: Vec<String>,
}

impl VocabularyOptions {
    /// Returns whether no vocabulary capture argument was supplied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bundle.is_none()
            && self.identity.is_none()
            && self.version.is_none()
            && self.encoding_version.is_none()
            && self.schema_version.is_none()
            && self.digest.is_none()
            && self.features.is_empty()
    }

    /// Verifies that every required captured vocabulary lock argument exists.
    pub fn validate(&self) -> Result<(), String> {
        if self.is_empty()
            || self.bundle.is_some()
                && self.identity.is_some()
                && self.version.is_some()
                && self.encoding_version.is_some()
                && self.schema_version.is_some()
                && self.digest.is_some()
        {
            Ok(())
        } else {
            Err("vocabulary capture requires bundle, identity, version, encoding, schema, and digest"
                .to_owned())
        }
    }
}

/// Fully parsed explicit command policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOptions {
    /// Selected host operation.
    pub kind: CommandKind,
    /// Explicit source path or standard-input sentinel.
    pub input: String,
    /// Explicit output path or standard-output sentinel.
    pub output: Option<String>,
    /// Whether an existing regular output may be atomically replaced.
    pub overwrite: bool,
    /// Whether cancellation is requested before capture starts.
    pub cancel_before_start: bool,
    /// Effective deterministic structural limits.
    pub limits: StructuralLimits,
    /// Optional exact captured vocabulary policy.
    pub vocabulary: VocabularyOptions,
}

/// A non-executing command parser outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseOutcome {
    /// Render general or command-specific help.
    Help(&'static str),
    /// Render the package version.
    Version,
    /// Execute one fully parsed command.
    Execute(Box<CommandOptions>),
}

/// Mutable numeric limit values while parsing command arguments.
struct LimitValues {
    /// Maximum source bytes.
    source_bytes: u64,
    /// Maximum diagnostics.
    diagnostics: u32,
    /// Maximum decoded string bytes.
    string_bytes: u64,
    /// Maximum exact-number digits.
    numeric_digits: u64,
    /// Maximum exact-number scale.
    numeric_scale: u64,
    /// Maximum declarations.
    declarations: u64,
    /// Maximum fields.
    record_fields: u64,
    /// Maximum nesting depth.
    nesting_depth: u64,
    /// Maximum list items.
    list_items: u64,
    /// Maximum traversal nodes.
    traversal_nodes: u64,
}

impl Default for LimitValues {
    /// Returns the reviewed bounded CLI defaults.
    fn default() -> Self {
        Self {
            source_bytes: constants::DEFAULT_SOURCE_BYTES,
            diagnostics: constants::DEFAULT_DIAGNOSTICS,
            string_bytes: constants::DEFAULT_STRING_BYTES,
            numeric_digits: constants::DEFAULT_NUMERIC_DIGITS,
            numeric_scale: constants::DEFAULT_NUMERIC_SCALE,
            declarations: constants::DEFAULT_DECLARATIONS,
            record_fields: constants::DEFAULT_RECORD_FIELDS,
            nesting_depth: constants::DEFAULT_NESTING_DEPTH,
            list_items: constants::DEFAULT_LIST_ITEMS,
            traversal_nodes: constants::DEFAULT_TRAVERSAL_NODES,
        }
    }
}

impl LimitValues {
    /// Constructs the checked shared structural limit contract.
    fn build(&self) -> Result<StructuralLimits, String> {
        StructuralLimits::new(self.source_bytes, self.diagnostics)
            .and_then(|limits| limits.with_string_bytes(self.string_bytes))
            .and_then(|limits| limits.with_numeric_digits(self.numeric_digits))
            .and_then(|limits| limits.with_numeric_scale(self.numeric_scale))
            .and_then(|limits| limits.with_declarations(self.declarations))
            .and_then(|limits| limits.with_record_fields(self.record_fields))
            .and_then(|limits| limits.with_nesting_depth(self.nesting_depth))
            .and_then(|limits| limits.with_list_items(self.list_items))
            .and_then(|limits| limits.with_traversal_nodes(self.traversal_nodes))
            .map_err(|_| "every limit must be greater than zero".to_owned())
    }
}

/// Parses command arguments without performing filesystem or process effects.
pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<ParseOutcome, String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    let Some(first) = arguments.first().map(String::as_str) else {
        return Err(format!(
            "a command is required; usage: {}",
            constants::USAGE
        ));
    };
    if first == constants::HELP {
        return Ok(ParseOutcome::Help(constants::USAGE));
    }
    if first == constants::VERSION {
        return Ok(ParseOutcome::Version);
    }
    let (kind, usage) = command_kind(first)?;
    if arguments
        .get(1)
        .is_some_and(|value| value == constants::HELP)
    {
        return Ok(ParseOutcome::Help(usage));
    }
    parse_command_arguments(&arguments, kind, usage)
}

/// Parses options and positional input for one already selected command.
fn parse_command_arguments(
    arguments: &[String],
    kind: CommandKind,
    usage: &'static str,
) -> Result<ParseOutcome, String> {
    let mut input = None;
    let mut output = None;
    let mut overwrite = false;
    let mut cancel_before_start = false;
    let mut limits = LimitValues::default();
    let mut vocabulary = VocabularyOptions::default();
    let mut index = 1;
    while index < arguments.len() {
        let argument = &arguments[index];
        match argument.as_str() {
            constants::OVERWRITE => overwrite = true,
            constants::CANCEL_BEFORE_START => cancel_before_start = true,
            constants::OUTPUT => set_once(
                &mut output,
                next_value(arguments, &mut index, argument)?,
                argument,
            )?,
            constants::VOCABULARY_BUNDLE => set_once(
                &mut vocabulary.bundle,
                next_value(arguments, &mut index, argument)?,
                argument,
            )?,
            constants::VOCABULARY_IDENTITY => set_once(
                &mut vocabulary.identity,
                next_value(arguments, &mut index, argument)?,
                argument,
            )?,
            constants::VOCABULARY_VERSION => set_once(
                &mut vocabulary.version,
                next_value(arguments, &mut index, argument)?,
                argument,
            )?,
            constants::VOCABULARY_ENCODING => set_once(
                &mut vocabulary.encoding_version,
                next_value(arguments, &mut index, argument)?,
                argument,
            )?,
            constants::VOCABULARY_SCHEMA => set_once(
                &mut vocabulary.schema_version,
                next_value(arguments, &mut index, argument)?,
                argument,
            )?,
            constants::VOCABULARY_DIGEST => set_once(
                &mut vocabulary.digest,
                next_value(arguments, &mut index, argument)?,
                argument,
            )?,
            constants::VOCABULARY_FEATURE => vocabulary
                .features
                .push(next_value(arguments, &mut index, argument)?),
            constants::MAX_SOURCE_BYTES => {
                limits.source_bytes = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_DIAGNOSTICS => {
                limits.diagnostics = parse_u32(arguments, &mut index, argument)?;
            }
            constants::MAX_STRING_BYTES => {
                limits.string_bytes = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_NUMERIC_DIGITS => {
                limits.numeric_digits = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_NUMERIC_SCALE => {
                limits.numeric_scale = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_DECLARATIONS => {
                limits.declarations = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_RECORD_FIELDS => {
                limits.record_fields = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_NESTING_DEPTH => {
                limits.nesting_depth = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_LIST_ITEMS => {
                limits.list_items = parse_u64(arguments, &mut index, argument)?;
            }
            constants::MAX_TRAVERSAL_NODES => {
                limits.traversal_nodes = parse_u64(arguments, &mut index, argument)?;
            }
            value if value.starts_with('-') && value != constants::STANDARD_STREAM => {
                return Err(format!("unknown option: {value}; usage: {usage}"));
            }
            value => set_once(&mut input, value.to_owned(), "source")?,
        }
        index += 1;
    }
    vocabulary.validate()?;
    validate_output_policy(kind, output.as_deref(), overwrite, usage)?;
    let input = input.ok_or_else(|| format!("source is required; usage: {usage}"))?;
    validate_standard_inputs(&input, &vocabulary)?;
    Ok(ParseOutcome::Execute(Box::new(CommandOptions {
        kind,
        input,
        output,
        overwrite,
        cancel_before_start,
        limits: limits.build()?,
        vocabulary,
    })))
}

/// Prevents two captured inputs from competing for the same standard stream.
fn validate_standard_inputs(input: &str, vocabulary: &VocabularyOptions) -> Result<(), String> {
    if input == constants::STANDARD_STREAM
        && vocabulary.bundle.as_deref() == Some(constants::STANDARD_STREAM)
    {
        Err("source and vocabulary bundle cannot both use standard input".to_owned())
    } else {
        Ok(())
    }
}

/// Resolves one stable command spelling and its usage.
fn command_kind(value: &str) -> Result<(CommandKind, &'static str), String> {
    match value {
        constants::COMPILE => Ok((CommandKind::Compile, constants::COMPILE_USAGE)),
        constants::VALIDATE => Ok((CommandKind::Validate, constants::VALIDATE_USAGE)),
        constants::FORMAT => Ok((CommandKind::Format, constants::FORMAT_USAGE)),
        _ => Err(format!(
            "unknown command: {value}; usage: {}",
            constants::USAGE
        )),
    }
}

/// Validates command-specific destination and replacement policy.
fn validate_output_policy(
    kind: CommandKind,
    output: Option<&str>,
    overwrite: bool,
    usage: &str,
) -> Result<(), String> {
    match kind {
        CommandKind::Compile | CommandKind::Format if output.is_none() => {
            Err(format!("output is required; usage: {usage}"))
        }
        CommandKind::Validate if output.is_some() || overwrite => {
            Err(format!("validate produces no output; usage: {usage}"))
        }
        CommandKind::Compile | CommandKind::Format
            if output == Some(constants::STANDARD_STREAM) && overwrite =>
        {
            Err(format!(
                "overwrite is invalid for standard output; usage: {usage}"
            ))
        }
        _ => Ok(()),
    }
}

/// Returns one required option value and advances the parser cursor.
fn next_value(arguments: &[String], index: &mut usize, option: &str) -> Result<String, String> {
    *index += 1;
    arguments
        .get(*index)
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or_else(|| format!("{option} requires a value"))
}

/// Sets one non-repeatable string argument exactly once.
fn set_once(target: &mut Option<String>, value: String, option: &str) -> Result<(), String> {
    if target.replace(value).is_some() {
        Err(format!("{option} may be supplied only once"))
    } else {
        Ok(())
    }
}

/// Parses one required positive unsigned 64-bit option value.
fn parse_u64(arguments: &[String], index: &mut usize, option: &str) -> Result<u64, String> {
    next_value(arguments, index, option)?
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{option} requires a positive integer"))
}

/// Parses one required positive unsigned 32-bit option value.
fn parse_u32(arguments: &[String], index: &mut usize, option: &str) -> Result<u32, String> {
    next_value(arguments, index, option)?
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{option} requires a positive integer"))
}

#[cfg(test)]
/// Tests the stable command parser without host effects.
mod tests {
    use super::{ParseOutcome, parse};
    use crate::constants;

    #[test]
    /// Verifies that the CLI shell accepts its help flag.
    fn package_shell_cli_accepts_help() {
        assert_eq!(
            parse([constants::HELP.to_owned()]),
            Ok(ParseOutcome::Help(constants::USAGE))
        );
    }

    #[test]
    /// Verifies that the CLI shell rejects a missing command.
    fn package_shell_cli_rejects_empty_arguments() {
        assert!(parse(Vec::new()).is_err());
    }

    #[test]
    /// Verifies incomplete vocabulary lock arguments fail before host acquisition.
    fn unit_cli_rejects_partial_vocabulary_capture_policy() {
        assert!(
            parse([
                constants::VALIDATE.to_owned(),
                constants::VOCABULARY_BUNDLE.to_owned(),
                "bundle.json".to_owned(),
                "source.neu".to_owned(),
            ])
            .is_err()
        );
    }
}
