// SPDX-License-Identifier: Apache-2.0

//! Shared human-readable automation output; machine-readable payloads bypass it.

use std::{
    fmt,
    io::{self, Write},
    path::Path,
    time::Duration,
};

/// Emits a categorized informational line without panicking on a closed pipe.
pub(crate) fn info(message: impl fmt::Display) {
    let _ = writeln!(io::stderr().lock(), "{} {message}", crate::constants::INFO);
}

/// Emits a categorized warning without panicking on a closed pipe.
pub(crate) fn warn(message: impl fmt::Display) {
    let _ = writeln!(io::stderr().lock(), "{} {message}", crate::constants::WARN);
}

/// Emits a categorized failure without panicking on a closed pipe.
pub fn error(message: impl fmt::Display) {
    let _ = writeln!(io::stderr().lock(), "{} {message}", crate::constants::ERROR);
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
