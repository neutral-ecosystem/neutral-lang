// SPDX-License-Identifier: Apache-2.0

//! Structural validation for the repository's GitHub Actions workflows.

use crate::constants;
use yaml_serde::Value;

/// Checks workflow behavior from parsed YAML and tokenized shell commands.
pub(crate) fn validate_workflows(
    ci: &str,
    release: &str,
    retained_workflows: &str,
) -> Result<(), String> {
    let ci = parse(ci, "ci.yml")?;
    let release = parse(release, "release.yml")?;
    validate_ci(&ci, retained_workflows)?;
    validate_release(&release)
}

/// Parses one workflow document with its filename in diagnostics.
fn parse(content: &str, name: &str) -> Result<Value, String> {
    yaml_serde::from_str(content).map_err(|error| format!("could not parse {name}: {error}"))
}

/// Returns a mapping field by its string key.
fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.as_mapping()?.get(Value::String(name.to_owned()))
}

/// Returns a YAML string scalar.
fn string(value: &Value) -> Option<&str> {
    value.as_str()
}

/// Iterates sequence items, or no items for another YAML type.
fn sequence(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_sequence().into_iter().flatten()
}

/// Iterates mapping entries, or no entries for another YAML type.
fn mapping(value: &Value) -> impl Iterator<Item = (&Value, &Value)> {
    value.as_mapping().into_iter().flat_map(|map| map.iter())
}

/// Returns the workflow's event trigger mapping.
fn events(workflow: &Value) -> Result<&Value, String> {
    field(workflow, "on").ok_or_else(|| "workflow has no event triggers".to_owned())
}

/// Converts a failed contract predicate into its diagnostic.
fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

/// Checks whether a named event is configured.
fn event_has(workflow: &Value, event: &str) -> bool {
    let Ok(events) = events(workflow) else {
        return false;
    };
    field(events, event).is_some()
}

/// Checks effective CI permissions at workflow and job scope.
fn ci_permissions_are_read_only(workflow: &Value) -> bool {
    let Some(root_permissions) = field(workflow, "permissions") else {
        return false;
    };
    if !read_permissions(root_permissions) {
        return false;
    }
    let Some(jobs) = field(workflow, "jobs") else {
        return false;
    };
    mapping(jobs).all(|(_, job)| {
        if explicitly_disabled(job) {
            return false;
        }
        let permissions = field(job, "permissions").unwrap_or(root_permissions);
        read_permissions(permissions)
    })
}

/// Reports permission scalars outside the accepted read-only values.
fn permission_writes(permissions: &Value) -> bool {
    mapping(permissions).any(|(_, permission)| !matches!(string(permission), Some("read" | "none")))
}

/// Requires a typed permissions map with contents read access.
fn read_permissions(permissions: &Value) -> bool {
    permissions.as_mapping().is_some()
        && field(permissions, "contents").and_then(string) == Some("read")
        && !permission_writes(permissions)
}

/// Recognizes Boolean or expression-wrapped false without depending on YAML whitespace.
fn explicitly_disabled(scope: &Value) -> bool {
    field(scope, "if").is_some_and(|value| {
        value.as_bool() == Some(false)
            || expression_tokens(value).is_some_and(|tokens| tokens == ["false"])
    })
}

/// Iterates steps that are not explicitly disabled.
fn steps(job: &Value) -> impl Iterator<Item = &Value> {
    field(job, "steps")
        .into_iter()
        .flat_map(sequence)
        .filter(|step| !explicitly_disabled(step))
}

/// Tokenizes one run script into shell invocations.
fn run_step(step: &Value) -> Result<Vec<Vec<String>>, String> {
    let Some(script) = field(step, "run").and_then(string) else {
        return Ok(Vec::new());
    };
    shell_commands(script)
}

/// Tokenizes supported line, continuation, and command-substitution forms.
fn shell_commands(script: &str) -> Result<Vec<Vec<String>>, String> {
    // shortcut: supports simple commands and known guards; extend validation before adopting shell control flow.
    let mut logical_lines = Vec::new();
    let mut pending = String::new();
    for line in script.lines() {
        let line = line.trim();
        if let Some(line) = line.strip_suffix('\\') {
            pending.push_str(line);
            pending.push(' ');
        } else {
            pending.push_str(line);
            logical_lines.push(std::mem::take(&mut pending));
        }
    }
    if !pending.is_empty() {
        logical_lines.push(pending);
    }
    let mut commands = Vec::new();
    for line in logical_lines {
        let mut current = Vec::new();
        let tokens =
            shlex::split(&line).ok_or_else(|| format!("malformed shell command: {line}"))?;
        if tokens
            .iter()
            .any(|token| token.contains("&&") || token.contains("||"))
        {
            return Err(
                "conditional shell chains are outside the supported workflow forms".to_owned(),
            );
        }
        if tokens.iter().any(|token| {
            token.contains("<<")
                || token.contains('`')
                || token.contains("()")
                || matches!(token.as_str(), "{" | "}")
        }) {
            return Err(
                "shell definitions and grouping are outside the supported workflow forms"
                    .to_owned(),
            );
        }
        for token in &tokens {
            if token == ";" {
                if !current.is_empty() {
                    commands.push(std::mem::take(&mut current));
                }
            } else {
                current.push(token.to_owned());
            }
        }
        if !current.is_empty() {
            commands.push(current);
        }
    }
    if commands.iter().any(|command| {
        matches!(
            command.first().map(String::as_str),
            Some(
                "if" | "then"
                    | "elif"
                    | "else"
                    | "fi"
                    | "for"
                    | "while"
                    | "until"
                    | "case"
                    | "esac"
                    | "do"
                    | "done"
                    | "select"
                    | "function"
                    | "!"
                    | "exit"
                    | "return"
            )
        )
    }) {
        return Err("shell control flow is outside the supported workflow forms".to_owned());
    }
    Ok(commands)
}

/// Parses substitutions only in the release guards' supported assignment/test forms.
fn guard_commands(step: &Value) -> Result<Vec<Vec<String>>, String> {
    let mut commands = run_step(step)?;
    let Some(script) = field(step, "run").and_then(string) else {
        return Ok(commands);
    };
    for line in script.lines().map(str::trim) {
        if assignment_name(line).is_some() || line.starts_with("test \"$(") {
            for nested in command_substitutions(line) {
                commands.extend(shell_commands(nested)?);
            }
        }
    }
    Ok(commands)
}

/// Finds a valid variable name in the supported quoted assignment form.
fn assignment_name(line: &str) -> Option<&str> {
    let (name, _) = line.split_once("=\"$(")?;
    let mut bytes = name.bytes();
    matches!(bytes.next()?, b'a'..=b'z' | b'A'..=b'Z' | b'_').then_some(())?;
    name.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        .then_some(name)
}

/// Returns assignment names paired with their parsed command substitutions.
fn shell_assignments(step: &Value) -> Result<Vec<(String, Vec<String>)>, String> {
    let Some(script) = field(step, "run").and_then(string) else {
        return Ok(Vec::new());
    };
    let mut assignments = Vec::new();
    for line in script.lines().map(str::trim) {
        let Some(name) = assignment_name(line) else {
            continue;
        };
        let substitutions = command_substitutions(line);
        let nested = substitutions
            .first()
            .ok_or_else(|| format!("malformed command assignment: {line}"))?;
        let tokens = shell_commands(nested)?;
        if let Some(command) = tokens.into_iter().next() {
            assignments.push((name.to_owned(), command));
        }
    }
    Ok(assignments)
}

/// Extracts simple `$()` expressions from the supported guard lines.
fn command_substitutions(script: &str) -> Vec<&str> {
    let mut substitutions = Vec::new();
    let mut rest = script;
    while let Some(start) = rest.find("$(") {
        let command = &rest[start + 2..];
        let Some(end) = command.find(')') else {
            break;
        };
        substitutions.push(&command[..end]);
        rest = &command[end + 1..];
    }
    substitutions
}

/// Tokenizes every active run step in a job.
fn commands(job: &Value) -> Result<Vec<Vec<String>>, String> {
    steps(job).try_fold(Vec::new(), |mut commands, step| {
        commands.extend(run_step(step)?);
        Ok(commands)
    })
}

/// Checks for a sha256sum verification invocation on `SHA256SUMS`.
fn has_checksum(commands: &[Vec<String>]) -> bool {
    commands.iter().any(|command| {
        command
            .first()
            .is_some_and(|name| name == constants::SHA256_COMMAND)
            && command.iter().any(|arg| arg == "--check" || arg == "-c")
            && command
                .iter()
                .any(|arg| arg == constants::RELEASE_CHECKSUM_FILE)
    })
}

/// Checks for a tokenized command with the requested leading arguments.
fn has_command(commands: &[Vec<String>], prefix: &[&str]) -> bool {
    commands.iter().any(|tokens| {
        tokens.len() >= prefix.len()
            && tokens
                .iter()
                .take(prefix.len())
                .map(String::as_str)
                .eq(prefix.iter().copied())
    })
}

/// Validates CI triggers, permissions, composition, and retained evidence.
fn validate_ci(workflow: &Value, retained_workflows: &str) -> Result<(), String> {
    require(event_has(workflow, "push"), "ci.yml must run on push")?;
    let push = field(events(workflow)?, "push").expect("push event was checked");
    let branches = field(push, "branches")
        .map(sequence)
        .ok_or_else(|| "ci.yml push must restrict branches".to_owned())?;
    let branches = branches.collect::<Vec<_>>();
    require(
        branches.len() == 1 && string(branches[0]) == Some(constants::RELEASE_BRANCH),
        "ci.yml must run only on the release branch",
    )?;
    require(
        !event_has(workflow, "pull_request"),
        "ci.yml must not run with pull request credentials",
    )?;
    require(
        !event_has(workflow, "pull_request_target"),
        "ci.yml must not run with pull request credentials",
    )?;
    require(
        ci_permissions_are_read_only(workflow),
        "ci.yml must declare effective read-only permissions",
    )?;
    let jobs = field(workflow, "jobs").ok_or_else(|| "ci.yml has no jobs".to_owned())?;
    let all_commands = mapping(jobs).try_fold(Vec::new(), |mut all, (_, job)| {
        all.extend(commands(job)?);
        Ok::<_, String>(all)
    })?;
    require(
        has_command(&all_commands, &["cargo", "xtask", "ci", "pr"]),
        "ci.yml must delegate to `cargo xtask ci pr`",
    )?;
    require(
        !has_command(&all_commands, &["cargo", "xtask", "docs"])
            && !has_command(&all_commands, &["cargo", "xtask", "quality"]),
        "ci.yml must use the single logged CI composition",
    )?;
    let retained = mapping(jobs).any(|(_, job)| {
        steps(job).any(|step| {
            let always = field(step, "if").is_some_and(|value| {
                expression_tokens(value).is_some_and(|tokens| tokens == ["always()"])
            });
            let uploader = field(step, "uses")
                .and_then(string)
                .is_some_and(|value| value.starts_with("actions/upload-artifact@"));
            let path = field(step, "with")
                .and_then(|with| field(with, "path"))
                .and_then(string)
                .is_some_and(|path| path.lines().any(|line| line.trim() == retained_workflows));
            let retention = field(step, "with")
                .and_then(|with| field(with, "retention-days"))
                .and_then(Value::as_u64)
                .is_some_and(|days| days > 0);
            always && uploader && path && retention
        })
    });
    require(
        retained,
        "ci.yml must retain workflow evidence on every outcome",
    )
}

/// Checks the supported exact push-only event expression.
fn push_only(value: &Value) -> bool {
    expression_tokens(value).is_some_and(|tokens| tokens == ["github.event_name", "==", "push"])
}

/// Tokenizes a plain or `${{ }}` expression without its outer delimiters.
fn expression_tokens(value: &Value) -> Option<Vec<String>> {
    let expression = string(value)?.trim();
    let expression = expression
        .strip_prefix("${{")
        .and_then(|value| value.strip_suffix("}}"))
        .unwrap_or(expression)
        .trim();
    shlex::split(expression)
}

/// Checks the supported tag-or-main checkout reference expression.
fn checkout_main_expression(value: &Value) -> bool {
    expression_tokens(value).is_some_and(|tokens| {
        tokens
            == [
                "github.event_name",
                "==",
                "push",
                "&&",
                "github.ref",
                "||",
                constants::RELEASE_BRANCH,
            ]
    })
}

/// Allows write access only to the contents scope of the publish job.
fn release_permissions_are_scoped(workflow: &Value) -> bool {
    let Some(root_permissions) = field(workflow, "permissions") else {
        return false;
    };
    let Some(jobs) = field(workflow, "jobs") else {
        return false;
    };
    read_permissions(root_permissions)
        && mapping(jobs).all(|(name, job)| {
            if explicitly_disabled(job) {
                return false;
            }
            let permissions = field(job, "permissions").unwrap_or(root_permissions);
            if string(name) == Some("publish") {
                permissions.as_mapping().is_some()
                    && field(permissions, "contents").and_then(string) == Some("write")
                    && mapping(permissions).all(|(name, permission)| {
                        if string(name) == Some("contents") {
                            string(permission) == Some("write")
                        } else {
                            matches!(string(permission), Some("read" | "none"))
                        }
                    })
            } else {
                read_permissions(permissions)
            }
        })
}

/// Returns the required qualify and publish jobs.
fn release_jobs(workflow: &Value) -> Result<(&Value, &Value), String> {
    let jobs = field(workflow, "jobs").ok_or_else(|| "release.yml has no jobs".to_owned())?;
    let qualify =
        field(jobs, "qualify").ok_or_else(|| "release.yml has no qualify job".to_owned())?;
    let publish =
        field(jobs, "publish").ok_or_else(|| "release.yml has no publish job".to_owned())?;
    Ok((qualify, publish))
}

/// Checks that one push-only guard compares the actual tag and remote branch before switching.
fn validates_main_preparation(step: &Value) -> Result<bool, String> {
    let tokens = guard_commands(step)?;
    let assignments = shell_assignments(step)?;
    let rev_list = assignments.iter().find(|(_, command)| {
        command.starts_with(&[constants::GIT_COMMAND.to_owned(), "rev-list".to_owned()])
            && command.iter().any(|arg| arg == "$RELEASE_TAG")
    });
    let main_head = assignments.iter().find(|(_, command)| {
        has_command(
            std::slice::from_ref(command),
            &[
                constants::GIT_COMMAND,
                "rev-parse",
                &format!(
                    "refs/remotes/{}/{}",
                    constants::RELEASE_REMOTE,
                    constants::RELEASE_BRANCH
                ),
            ],
        )
    });
    let equality = tokens.iter().any(|command| {
        command.first().is_some_and(|name| name == "test")
            && rev_list.is_some_and(|(name, _)| {
                command.get(1).is_some_and(|arg| arg == &format!("${name}"))
            })
            && command.get(2).is_some_and(|arg| arg == "=")
            && main_head.is_some_and(|(name, _)| {
                command.get(3).is_some_and(|arg| arg == &format!("${name}"))
            })
    });
    let switch = tokens.iter().any(|command| {
        command
            .first()
            .is_some_and(|name| name == constants::GIT_COMMAND)
            && command.get(1).is_some_and(|arg| arg == "switch")
            && (command.get(2).is_some_and(|arg| arg == "-C")
                || command.get(2).is_some_and(|arg| arg == "--force-create"))
            && command
                .get(3)
                .is_some_and(|branch| branch == constants::RELEASE_BRANCH)
            && [rev_list, main_head]
                .into_iter()
                .flatten()
                .any(|(name, _)| command.get(4).is_some_and(|arg| arg == &format!("${name}")))
    });
    Ok(rev_list.is_some() && main_head.is_some() && equality && switch)
}

/// Validates tag identity, workspace-version agreement and dispatch checkout preparation.
fn validate_tag_preparation(qualify: &Value) -> Result<(), String> {
    let prep = steps(qualify)
        .filter(|step| push_only(field(step, "if").unwrap_or(&Value::Null)))
        .collect::<Vec<_>>();
    let mut main_prep = false;
    for step in &prep {
        main_prep |= validates_main_preparation(step)?;
    }
    require(
        main_prep,
        "release push must verify the tag is main HEAD and prepare the main branch",
    )?;
    let tag_agreement = prep.iter().try_fold(false, |valid, step| {
        let tokens = guard_commands(step)?;
        Ok::<_, String>(
            valid
                || (has_command(&tokens, &["cargo", "xtask", "release", "tag"])
                    && tokens.iter().any(|command| {
                        command.first().is_some_and(|name| name == "test")
                            && command.iter().any(|arg| arg == "=")
                            && command.iter().any(|arg| arg == "$RELEASE_TAG")
                    })),
        )
    })?;
    require(
        tag_agreement,
        "release push must match the tag to the workspace release plan",
    )?;
    let checkout_main = steps(qualify).any(|step| {
        field(step, "uses")
            .and_then(string)
            .is_some_and(|action| action.starts_with("actions/checkout@"))
            && field(step, "with")
                .and_then(|with| field(with, "ref"))
                .is_some_and(checkout_main_expression)
    });
    require(
        checkout_main,
        "release workflow dispatch must qualify the configured main branch",
    )?;
    Ok(())
}

/// Ties the verified draft invocation to the actual event tag through effective step environment.
fn creates_event_tag_draft(step: &Value, job: &Value, workflow: &Value) -> Result<bool, String> {
    let tag = [step, job, workflow]
        .into_iter()
        .find_map(|scope| field(scope, "env").and_then(|env| field(env, "RELEASE_TAG")));
    if !tag.is_some_and(|value| {
        expression_tokens(value).is_some_and(|tokens| tokens == ["github.ref_name"])
    }) {
        return Ok(false);
    }
    Ok(run_step(step)?.iter().any(|command| {
        command
            .iter()
            .take(4)
            .map(String::as_str)
            .eq(["gh", "release", "create", "$RELEASE_TAG"])
            && command.iter().any(|arg| arg == "--verify-tag")
            && command.iter().any(|arg| arg == "--draft")
            && !command.iter().any(|arg| {
                [
                    "--verify-tag=false",
                    "--no-verify-tag",
                    "--draft=false",
                    "--no-draft",
                ]
                .contains(&arg.as_str())
            })
    }))
}

/// Validates tag qualification and guarded draft publication.
fn validate_release(workflow: &Value) -> Result<(), String> {
    let events = events(workflow)?;
    require(
        mapping(events)
            .all(|(event, _)| matches!(string(event), Some("push" | "workflow_dispatch")))
            && event_has(workflow, "workflow_dispatch"),
        "release.yml must use tag pushes and workflow dispatch only",
    )?;
    let push =
        field(events, "push").ok_or_else(|| "release.yml must run on tag pushes".to_owned())?;
    require(
        field(push, "branches").is_none(),
        "release.yml must not run on branch pushes",
    )?;
    let tags = field(push, "tags")
        .map(sequence)
        .ok_or_else(|| "release.yml push must filter release tags".to_owned())?;
    let tags = tags.collect::<Vec<_>>();
    require(
        tags.len() == 1 && string(tags[0]) == Some("v*"),
        "release.yml must select v* tags",
    )?;
    let (qualify, publish) = release_jobs(workflow)?;
    require(
        release_permissions_are_scoped(workflow),
        "release permissions must be read-only except for publication",
    )?;
    let qualify_commands = commands(qualify)?;
    require(
        has_command(&qualify_commands, &["cargo", "xtask", "release", "qualify"]),
        "release.yml must delegate to `cargo xtask release qualify`",
    )?;
    require(
        has_checksum(&qualify_commands),
        "release qualification must verify SHA256SUMS",
    )?;
    let artifact_retained = steps(qualify).any(|step| {
        field(step, "uses")
            .and_then(string)
            .is_some_and(|action| action.starts_with("actions/upload-artifact@"))
            && field(step, "with")
                .and_then(|with| field(with, "retention-days"))
                .and_then(Value::as_u64)
                .is_some_and(|days| days > 0)
    });
    require(
        artifact_retained,
        "release qualification must retain uploaded artifacts",
    )?;
    validate_tag_preparation(qualify)?;
    let needs_qualify = field(publish, "needs").is_some_and(|needs| {
        string(needs) == Some("qualify")
            || sequence(needs).any(|need| string(need) == Some("qualify"))
    });
    require(needs_qualify, "release publication must depend on qualify")?;
    require(
        field(publish, "if").is_some_and(push_only),
        "release publication must be push-only",
    )?;
    let publish_commands = commands(publish)?;
    let create = steps(publish).try_fold(false, |seen, step| {
        let valid = creates_event_tag_draft(step, publish, workflow)?;
        Ok::<_, String>(seen || valid)
    })?;
    require(
        create,
        "release publication must create a verified draft in one `gh release create` invocation",
    )?;
    require(
        has_checksum(&publish_commands),
        "publication must verify downloaded SHA256SUMS",
    )
}
