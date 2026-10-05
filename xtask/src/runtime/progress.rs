// SPDX-License-Identifier: Apache-2.0

//! Estimated command progress without interfering with retained tool reports.

use std::{
    io::{self, IsTerminal, Write},
    process::{Child, ExitStatus},
    thread,
    time::{Duration, Instant},
};

/// Waits for a child, refreshing terminal progress or emitting periodic log lines.
pub(crate) fn wait(child: &mut Child, label: &str, budget: Option<u64>) -> io::Result<ExitStatus> {
    let start = Instant::now();
    let terminal = io::stderr().is_terminal();
    let mut previous = None;
    loop {
        if let Some(status) = child.try_wait()? {
            let mut output = io::stderr().lock();
            if terminal && previous.is_some() {
                let _ = write!(output, "\r\x1b[K");
            }
            if status.success()
                && let Some(budget) = budget
            {
                let _ = writeln!(
                    output,
                    "{}",
                    render_status(label, start.elapsed().as_secs(), budget, true)
                );
            }
            let _ = output.flush();
            return Ok(status);
        }
        let elapsed = start.elapsed().as_secs();
        if previous.is_none_or(|last| elapsed >= last + if terminal { 1 } else { 10 }) {
            let line = match budget {
                Some(budget) => render(label, elapsed, budget),
                None => crate::output::progress(
                    &format!(
                        "{label} | elapsed {} | reports are being captured",
                        crate::output::duration(Duration::from_secs(elapsed))
                    ),
                    false,
                ),
            };
            let mut output = io::stderr().lock();
            if terminal {
                let _ = write!(output, "\r{line}\x1b[K");
            } else {
                let _ = writeln!(output, "{line}");
            }
            let _ = output.flush();
            previous = Some(elapsed);
        }
        thread::sleep(Duration::from_millis(250));
    }
}

/// Formats budget progress, keeping completion distinct from child success.
fn render(label: &str, elapsed: u64, budget: u64) -> String {
    render_status(label, elapsed, budget, false)
}

/// Formats a full bar only for confirmed successful child completion.
fn render_status(label: &str, elapsed: u64, budget: u64, completed: bool) -> String {
    const WIDTH: usize = 20;
    let percent = if completed {
        100
    } else {
        ((u128::from(elapsed) * 100) / u128::from(budget.max(1))).min(99)
    };
    let filled = percent as usize * WIDTH / 100;
    let remaining = budget.saturating_sub(elapsed);
    let estimate = if completed {
        "tool complete; evidence validation follows".to_owned()
    } else if remaining == 0 {
        "budget elapsed; waiting for tool (build/startup may add time)".to_owned()
    } else {
        format!(
            "ETA ~{}m {:02}s + build/startup overhead",
            remaining / 60,
            remaining % 60
        )
    };
    crate::output::progress(
        &format!(
            "{label} [{}{}] {percent}% | elapsed {}m {:02}s | {estimate}",
            "=".repeat(filled),
            " ".repeat(WIDTH - filled),
            elapsed / 60,
            elapsed % 60
        ),
        completed,
    )
}

#[cfg(test)]
#[path = "../../tests/unit/progress.rs"]
mod tests;
