// SPDX-License-Identifier: Apache-2.0

//! Estimated command progress without interfering with retained tool reports.

use std::{
    io::{self, IsTerminal, Write},
    process::{Child, ExitStatus},
    thread,
    time::{Duration, Instant},
};

/// Waits for a child, refreshing terminal progress or emitting periodic log lines.
pub(super) fn wait(child: &mut Child, label: &str, budget: u64) -> io::Result<ExitStatus> {
    let start = Instant::now();
    let terminal = io::stderr().is_terminal();
    let mut previous = None;
    loop {
        let elapsed = start.elapsed().as_secs();
        if previous.is_none_or(|last| elapsed >= last + if terminal { 1 } else { 10 }) {
            let line = render(label, elapsed, budget);
            let mut output = io::stderr().lock();
            if terminal {
                let _ = write!(output, "\r{line}\x1b[K");
            } else {
                let _ = writeln!(output, "{line}");
            }
            let _ = output.flush();
            previous = Some(elapsed);
        }
        if let Some(status) = child.try_wait()? {
            let mut output = io::stderr().lock();
            if terminal {
                let _ = writeln!(output);
            }
            let _ = writeln!(
                output,
                "{} fuzz {label}: {status}; command elapsed {}s",
                crate::constants::INFO,
                start.elapsed().as_secs()
            );
            return Ok(status);
        }
        thread::sleep(Duration::from_millis(250));
    }
}

/// Formats budget progress, keeping completion distinct from child success.
fn render(label: &str, elapsed: u64, budget: u64) -> String {
    const WIDTH: usize = 20;
    let percent = ((u128::from(elapsed) * 100) / u128::from(budget.max(1))).min(99);
    let filled = percent as usize * WIDTH / 100;
    let remaining = budget.saturating_sub(elapsed);
    let estimate = if remaining == 0 {
        "budget elapsed; waiting for tool (build/startup may add time)".to_owned()
    } else {
        format!(
            "ETA ~{}m {:02}s + build/startup overhead",
            remaining / 60,
            remaining % 60
        )
    };
    format!(
        "{} fuzz {label} [{}{}] {percent}% | elapsed {}m {:02}s | {estimate}",
        crate::constants::INFO,
        "=".repeat(filled),
        " ".repeat(WIDTH - filled),
        elapsed / 60,
        elapsed % 60
    )
}

#[cfg(test)]
#[path = "../tests/unit/progress.rs"]
mod tests;
