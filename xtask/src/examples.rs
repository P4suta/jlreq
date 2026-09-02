// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Every documentation example compiles, and the runnable ones run.
//!
//! The crate READMEs are already compiled by rustdoc through
//! `#![doc = include_str!]`, but the root `README.md` and `docs/guide.ja.md`
//! are rendered by GitHub and crates.io without any compiler ever reading
//! them — the classic way documentation rots. This gate extracts every Rust
//! fence from those two files, synthesizes a scratch crate under
//! `target/doc-examples/` with a path dependency on the facade, compiles each
//! fence as its own binary, and executes the ones not marked `no_run` against
//! the packaged fixture font, so the documented programs demonstrably work.
//!
//! Fences may be named by an HTML comment on a preceding line:
//! `<!-- jlreq-example: NAME -->`. The same name appearing in more than one
//! checked file — the crate README included — requires the fence bodies to be
//! byte-identical, which is what keeps the root README's claim of showing
//! "the same example" as the compiled doctest mechanically true.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use crate::shared::{Gate, workspace_root};

pub(crate) const GATE: Gate = Gate {
    name: "examples",
    purpose: "every documentation example compiles and the runnable ones run against the fixture font",
    reference: "docs/design/api-spine.md",
    run,
};

/// Files whose fences are compiled, and run unless marked `no_run`.
const SOURCES: &[&str] = &["README.md", "docs/guide.ja.md"];

/// Files participating in shared-fence identity checks. The crate README is
/// compiled by rustdoc already, so it is only cross-checked here.
const SYNC_SOURCES: &[&str] = &["README.md", "crates/jlreq/README.md", "docs/guide.ja.md"];

/// The marker naming the fence that follows it.
const NAME_MARKER_PREFIX: &str = "<!-- jlreq-example: ";
const NAME_MARKER_SUFFIX: &str = " -->";

/// How a fence participates in the gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Run,
    CompileOnly,
    Skip,
}

#[derive(Debug)]
struct Fence {
    file: &'static str,
    line: usize,
    name: String,
    body: String,
    mode: Mode,
}

fn run(_arguments: &[String]) -> io::Result<Vec<String>> {
    let root = workspace_root()?;
    let mut violations = Vec::new();
    let mut fences = Vec::new();
    for file in SYNC_SOURCES {
        let text = fs::read_to_string(root.join(file))?;
        extract_fences(file, &text, &mut fences, &mut violations);
    }
    check_shared_identity(&fences, &mut violations);

    let checked: Vec<&Fence> = fences
        .iter()
        .filter(|fence| SOURCES.contains(&fence.file) && fence.mode != Mode::Skip)
        .collect();
    // A gate that can pass by having nothing to check is not a gate: each
    // documented entry point must keep at least one executed program.
    for file in SOURCES {
        if !checked
            .iter()
            .any(|fence| fence.file == *file && fence.mode == Mode::Run)
        {
            violations.push(format!(
                "{file}: no executable Rust example remains; each documented entry point must keep at least one program this gate runs"
            ));
        }
    }
    for fence in &checked {
        if !fence.body.contains("fn main") {
            violations.push(format!(
                "{}:{}: example `{}` is not a complete program (no `fn main`); \
                 documentation examples must show everything they need",
                fence.file, fence.line, fence.name
            ));
        }
    }

    let mut built = 0_usize;
    let mut executed = 0_usize;
    if violations.is_empty() && !checked.is_empty() {
        build_and_run(&root, &checked, &mut built, &mut executed, &mut violations)?;
    }
    let shared = shared_names(&fences).len();
    println!(
        "examples: extracted {} Rust fence(s) from {} file(s); built {built}, executed {executed} \
         against the fixture font, verified {shared} shared fence name(s) identical",
        checked.len(),
        SOURCES.len(),
    );
    Ok(violations)
}

fn extract_fences(
    file: &'static str,
    text: &str,
    fences: &mut Vec<Fence>,
    violations: &mut Vec<String>,
) {
    let mut pending_name: Option<String> = None;
    let mut open: Option<(usize, Mode, String)> = None;
    let mut anonymous = 0_usize;
    for (index, line) in text.lines().enumerate() {
        let number = index.saturating_add(1);
        let trimmed = line.trim();
        if let Some((start, mode, body)) = open.take() {
            if trimmed == "```" {
                let name = pending_name.take().unwrap_or_else(|| {
                    anonymous = anonymous.saturating_add(1);
                    format!("{file}-{anonymous}")
                });
                // Names collide globally, not per file: two fences differing
                // only in punctuation would otherwise share one scratch
                // binary and silently overwrite each other.
                let collision = fences
                    .iter()
                    .find(|fence| binary_name(&fence.name) == binary_name(&name))
                    .filter(|other| other.file == file || other.body != body);
                if let Some(other) = collision {
                    violations.push(format!(
                        "{file}:{start}: example name `{name}` collides with `{}` at {}:{}",
                        other.name, other.file, other.line
                    ));
                }
                fences.push(Fence {
                    file,
                    line: start,
                    name,
                    body,
                    mode,
                });
            } else {
                let mut body = body;
                body.push_str(line);
                body.push('\n');
                open = Some((start, mode, body));
            }
            continue;
        }
        if let Some(info) = trimmed.strip_prefix("```") {
            open = Some((number, fence_mode(info), String::new()));
            continue;
        }
        let stripped = trimmed
            .strip_prefix(NAME_MARKER_PREFIX)
            .and_then(|rest| rest.strip_suffix(NAME_MARKER_SUFFIX));
        if let Some(name) = stripped {
            pending_name = Some(name.to_owned());
        } else if !trimmed.is_empty() {
            pending_name = None;
        }
    }
    if let Some((start, _, _)) = open {
        violations.push(format!("{file}:{start}: unterminated code fence"));
    }
}

/// Read the fence info string the way rustdoc does: tokens split on
/// whitespace and commas.
fn fence_mode(info: &str) -> Mode {
    let tokens: Vec<&str> = info
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.first() != Some(&"rust") {
        return Mode::Skip;
    }
    if tokens.contains(&"ignore") {
        Mode::Skip
    } else if tokens.contains(&"no_run") {
        Mode::CompileOnly
    } else {
        Mode::Run
    }
}

fn shared_names(fences: &[Fence]) -> BTreeMap<&str, Vec<&Fence>> {
    let mut by_name: BTreeMap<&str, Vec<&Fence>> = BTreeMap::new();
    for fence in fences {
        by_name.entry(fence.name.as_str()).or_default().push(fence);
    }
    by_name.retain(|_, group| group.len() > 1);
    by_name
}

fn check_shared_identity(fences: &[Fence], violations: &mut Vec<String>) {
    for (name, group) in shared_names(fences) {
        let Some(first) = group.first() else {
            continue;
        };
        for other in group.iter().skip(1) {
            if other.body != first.body {
                violations.push(format!(
                    "shared example `{name}` differs between {}:{} and {}:{}; \
                     the copies must stay byte-identical",
                    first.file, first.line, other.file, other.line
                ));
            }
        }
    }
}

fn build_and_run(
    root: &Path,
    checked: &[&Fence],
    built: &mut usize,
    executed: &mut usize,
    violations: &mut Vec<String>,
) -> io::Result<()> {
    let scratch = root.join("target").join("doc-examples");
    let sources = scratch.join("src").join("bin");
    fs::create_dir_all(&sources)?;
    // Remove bins from renamed or deleted fences so stale programs cannot
    // keep the gate green.
    for entry in fs::read_dir(&sources)? {
        fs::remove_file(entry?.path())?;
    }
    // The empty [workspace] table detaches the scratch crate from the
    // repository workspace it physically sits inside.
    fs::write(
        scratch.join("Cargo.toml"),
        "# Generated by `cargo run -p xtask -- examples`; not committed.\n\
         [package]\n\
         name = \"doc-examples\"\n\
         version = \"0.0.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\n\
         [workspace]\n\n\
         [dependencies]\n\
         jlreq = { path = \"../../crates/jlreq\" }\n\
         font-test-data = \"=0.9.1\"\n",
    )?;
    fs::write(
        sources.join("doc_example_fixture_font.rs"),
        "fn main() -> Result<(), std::io::Error> {\n\
             let path = std::env::args().nth(1).expect(\"destination path\");\n\
             std::fs::write(path, font_test_data::NOTO_SANS_JP_CFF)\n\
         }\n",
    )?;
    for fence in checked {
        fs::write(
            sources.join(format!("{}.rs", binary_name(&fence.name))),
            &fence.body,
        )?;
    }

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let target_dir = root.join("target").join("doc-examples-target");
    let compilation = Command::new(&cargo)
        .arg("build")
        .arg("--bins")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(scratch.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()?;
    if !compilation.status.success() {
        violations.push(format!(
            "the documentation examples do not compile:\n{}",
            String::from_utf8_lossy(&compilation.stderr)
        ));
        return Ok(());
    }
    *built = checked.len();

    let fixture = scratch.join("NotoSansJP-fixture.otf");
    let binaries = target_dir.join("debug");
    let suffix = std::env::consts::EXE_SUFFIX;
    let writer = binaries.join(format!("doc_example_fixture_font{suffix}"));
    let wrote = Command::new(&writer).arg(&fixture).output()?;
    if !wrote.status.success() {
        violations.push(format!(
            "the fixture font could not be written:\n{}",
            String::from_utf8_lossy(&wrote.stderr)
        ));
        return Ok(());
    }

    for fence in checked {
        if fence.mode != Mode::Run {
            continue;
        }
        let program = binaries.join(format!("{}{suffix}", binary_name(&fence.name)));
        let output = Command::new(&program).arg(&fixture).output()?;
        if output.status.success() {
            *executed = executed.saturating_add(1);
        } else {
            violations.push(format!(
                "{}:{}: example `{}` failed at runtime:\n{}",
                fence.file,
                fence.line,
                fence.name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    Ok(())
}

fn binary_name(name: &str) -> String {
    let mut result = String::from("doc_example_");
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
        } else {
            result.push('_');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(text: &str) -> (Vec<Fence>, Vec<String>) {
        let mut fences = Vec::new();
        let mut violations = Vec::new();
        extract_fences("README.md", text, &mut fences, &mut violations);
        (fences, violations)
    }

    #[test]
    fn fence_modes_read_the_info_string_like_rustdoc() {
        assert_eq!(fence_mode("rust"), Mode::Run);
        assert_eq!(fence_mode("rust no_run"), Mode::CompileOnly);
        assert_eq!(fence_mode("rust,no_run"), Mode::CompileOnly);
        assert_eq!(fence_mode("rust ignore"), Mode::Skip);
        assert_eq!(fence_mode("rust,ignore,no_run"), Mode::Skip);
        assert_eq!(fence_mode("sh"), Mode::Skip);
        assert_eq!(fence_mode("toml"), Mode::Skip);
        assert_eq!(fence_mode(""), Mode::Skip);
        assert_eq!(fence_mode("text rust"), Mode::Skip);
    }

    #[test]
    fn extraction_names_bodies_and_modes_are_exact() {
        let text = "\
intro

<!-- jlreq-example: quickstart -->
```rust
fn main() {}
```

prose between resets the marker

<!-- jlreq-example: unused -->

```rust no_run
fn main() { let _ = 1; }
```

```sh
cargo add jlreq
```
";
        let (fences, violations) = extract(text);
        assert!(violations.is_empty(), "{violations:?}");
        assert_eq!(fences.len(), 3);
        assert_eq!(fences[0].name, "quickstart");
        assert_eq!(fences[0].mode, Mode::Run);
        assert_eq!(fences[0].body, "fn main() {}\n");
        assert_eq!(fences[0].line, 4);
        assert_eq!(
            fences[1].name, "unused",
            "a blank-line gap keeps the marker"
        );
        assert_eq!(fences[1].mode, Mode::CompileOnly);
        assert_eq!(fences[2].mode, Mode::Skip);
    }

    #[test]
    fn malformed_inputs_are_reported_not_ignored() {
        let (_, violations) = extract("```rust\nfn main() {}\n");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("unterminated"));

        let duplicated = "\
<!-- jlreq-example: twice -->
```rust
fn main() {}
```
<!-- jlreq-example: twice -->
```rust
fn main() {}
```
";
        let (fences, violations) = extract(duplicated);
        assert_eq!(fences.len(), 2);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("collides with"), "{violations:?}");
    }

    #[test]
    fn shared_fences_must_stay_byte_identical() {
        let mut fences = Vec::new();
        let mut violations = Vec::new();
        extract_fences(
            "README.md",
            "<!-- jlreq-example: shared -->\n```rust\nfn main() {}\n```\n",
            &mut fences,
            &mut violations,
        );
        extract_fences(
            "docs/guide.ja.md",
            "<!-- jlreq-example: shared -->\n```rust no_run\nfn main() {}\n```\n",
            &mut fences,
            &mut violations,
        );
        check_shared_identity(&fences, &mut violations);
        assert!(
            violations.is_empty(),
            "info strings may differ: {violations:?}"
        );

        let mut divergent = Vec::new();
        extract_fences(
            "crates/jlreq/README.md",
            "<!-- jlreq-example: shared -->\n```rust\nfn main() { let _ = 2; }\n```\n",
            &mut divergent,
            &mut violations,
        );
        fences.append(&mut divergent);
        check_shared_identity(&fences, &mut violations);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("shared example `shared` differs"));
    }

    #[test]
    fn binary_names_are_sanitized_and_prefixed() {
        assert_eq!(binary_name("quickstart"), "doc_example_quickstart");
        assert_eq!(binary_name("guide.ja-1"), "doc_example_guide_ja_1");
    }

    #[test]
    fn the_gate_holds_over_this_repository() {
        assert_eq!(run(&[]).unwrap(), Vec::<String>::new());
    }
}
