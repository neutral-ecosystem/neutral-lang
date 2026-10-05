// SPDX-License-Identifier: Apache-2.0

//! Shared human-readable automation output; machine-readable payloads bypass it.

use std::{
    env, fmt,
    io::{self, IsTerminal, Write},
    path::Path,
    time::Duration,
};

/// Emits a categorized informational line without panicking on a closed pipe.
pub(crate) fn info(message: impl fmt::Display) {
    emit(crate::constants::INFO, &message.to_string());
}

/// Emits a categorized warning without panicking on a closed pipe.
pub(crate) fn warn(message: impl fmt::Display) {
    emit(crate::constants::WARN, &message.to_string());
}

/// Emits a categorized failure without panicking on a closed pipe.
pub fn error(message: impl fmt::Display) {
    emit(crate::constants::ERROR, &message.to_string());
}

/// Writes a uniformly formatted human line on stderr with optional terminal color.
fn emit(category: &str, message: &str) {
    let line = format_line(category, message, color_enabled());
    let _ = writeln!(io::stderr().lock(), "{line}");
}

/// Resolves the shared Cargo/nextest terminal color policy without mutating global state.
fn color_enabled() -> bool {
    color_policy(
        io::stderr().is_terminal(),
        env::var("TERM").ok().as_deref(),
        env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
        env::var(crate::constants::CARGO_TERM_COLOR_ENV)
            .ok()
            .as_deref(),
    )
}

/// Honors explicit color controls while keeping redirected and dumb terminals plain.
fn color_policy(
    terminal: bool,
    term: Option<&str>,
    no_color: bool,
    cargo_color: Option<&str>,
) -> bool {
    if no_color {
        return false;
    }
    match cargo_color {
        Some("always") => true,
        Some("never") => false,
        _ => terminal && term != Some("dumb"),
    }
}

/// Propagates the resolved human-output color policy to inherited Cargo and nextest streams.
pub(crate) fn child_color() -> &'static str {
    if color_enabled() { "always" } else { "never" }
}

/// Renders a progress row through the same category, alignment, and color palette.
pub(crate) fn progress(message: &str, completed: bool) -> String {
    format_row(
        crate::constants::INFO,
        if completed { "PASS" } else { "RUN" },
        message,
        color_enabled(),
    )
}

/// Extracts common human status labels without changing script-facing payloads.
fn format_line(category: &str, message: &str, color: bool) -> String {
    if category == crate::constants::ERROR {
        return format_row(category, "FAIL", message, color);
    }
    if category == crate::constants::WARN {
        return format_row(category, "WARN", message, color);
    }
    if let Some(command) = message.strip_prefix("command: ") {
        return format_row(category, "CMD", command, color);
    }
    if let Some((label, status)) = message.split_once(": ") {
        for (word, action) in [
            ("start", "START"),
            ("pass", "PASS"),
            ("valid", "PASS"),
            ("approved", "PASS"),
            ("evidence verified", "PASS"),
        ] {
            if let Some(suffix) = status.strip_prefix(word)
                && (suffix.is_empty() || suffix.starts_with([' ', ';']))
            {
                return format_row(category, action, &format!("{label}{suffix}"), color);
            }
        }
        if matches!(
            label,
            "reports"
                | "receipt"
                | "workflow log"
                | "coverage HTML"
                | "coverage JSON"
                | "workspace documentation"
                | "package assembled"
                | "quality evaluation"
                | "version plan"
        ) {
            return format_row(category, "FILE", message, color);
        }
    }
    format_row(category, "INFO", message, color)
}

/// Applies fixed-width labels and semantic colors, never relying on color alone.
fn format_row(category: &str, action: &str, message: &str, color: bool) -> String {
    let row = format!("{category} {action:<5} {message}");
    if !color {
        return row;
    }
    let code = match action {
        "PASS" => "32",
        "FAIL" => "31",
        "WARN" => "33",
        "START" | "RUN" => "36",
        "CMD" | "FILE" => "2",
        _ => "34",
    };
    format!("\x1b[{code}m{category} {action:<5}\x1b[0m {message}")
}

/// Formats elapsed time in compact units instead of large millisecond counts.
pub(crate) fn duration(value: Duration) -> String {
    let seconds = value.as_secs();
    if seconds >= 3600 {
        format!(
            "{}h {:02}m {:02}s",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else if seconds >= 60 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else if seconds > 0 {
        format!("{:.1}s", value.as_secs_f64())
    } else {
        format!("{}ms", value.as_millis())
    }
}

/// Displays workspace-relative paths while preserving external paths verbatim.
pub(crate) fn path(value: &Path) -> String {
    crate::workspace_root()
        .ok()
        .and_then(|root| value.strip_prefix(root).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| value.to_path_buf())
        .display()
        .to_string()
}

/// Presents a copyable command with path shortening and POSIX argument quoting.
pub(crate) fn command(program: &str, arguments: &[&str]) -> String {
    std::iter::once(program)
        .chain(arguments.iter().copied())
        .map(|value| quote(&path(Path::new(value))))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quotes whitespace and shell metacharacters without changing argument contents.
fn quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_./:=-,+".contains(&byte))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
#[path = "../tests/unit/output.rs"]
mod tests;
