// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Repository-wide checks that do not belong to a Cargo package.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::shared::{self, Gate};

/// The repository-hygiene gate exposed by the dispatcher.
pub(crate) const GATE: Gate = Gate {
    name: "repository",
    purpose: "tracked UTF-8 files use LF and local Markdown links resolve",
    reference: "CONTRIBUTING.md",
    run,
};

fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    if let Some(first) = arguments.first() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the repository gate takes no arguments; got `{first}`"),
        ));
    }

    let root = shared::workspace_root()?;
    let files = tracked_files(&root)?;
    let mut utf8_files = 0_usize;
    let mut documents = 0_usize;
    let mut links = 0_usize;
    let mut violations = Vec::new();
    for file in &files {
        let bytes = fs::read(file)?;
        let Ok(source) = std::str::from_utf8(&bytes) else {
            continue;
        };
        utf8_files = utf8_files.saturating_add(1);
        let relative = shared::relative_name(file, &root);
        if source.contains('\r') {
            violations.push(format!(
                "{relative}: contains CR; tracked UTF-8 files use LF"
            ));
        }
        if file.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        documents = documents.saturating_add(1);
        for link in local_links(source) {
            links = links.saturating_add(1);
            if let Some(message) = unresolved_link(&root, file, &link.target) {
                violations.push(format!("{relative}:{}: {message}", link.line));
            }
        }
    }
    println!(
        "repository: examined {utf8_files} tracked UTF-8 file(s) and {links} local link(s) \
         in {documents} Markdown file(s)",
    );
    Ok(violations)
}

fn tracked_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let mut documents = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| root.join(String::from_utf8_lossy(path).as_ref()))
        .collect::<Vec<_>>();
    documents.sort();
    Ok(documents)
}

#[derive(Debug, PartialEq, Eq)]
struct Link {
    line: usize,
    target: String,
}

fn local_links(source: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut fenced = false;
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }

        let mut rest = line;
        while let Some((_, after_open)) = rest.split_once("](") {
            let Some((raw_target, after_close)) = after_open.split_once(')') else {
                break;
            };
            rest = after_close;
            let target = markdown_target(raw_target);
            if is_local_target(target) {
                links.push(Link {
                    line: index.saturating_add(1),
                    target: target.to_owned(),
                });
            }
        }
        if let Some((label, raw_target)) = line.split_once("]:")
            && label.trim_start().starts_with('[')
        {
            let target = markdown_target(raw_target);
            if is_local_target(target) {
                links.push(Link {
                    line: index.saturating_add(1),
                    target: target.to_owned(),
                });
            }
        }
    }
    links
}

fn markdown_target(raw: &str) -> &str {
    let trimmed = raw.trim_start();
    if let Some(opened) = trimmed.strip_prefix('<')
        && let Some((target, _)) = opened.split_once('>')
    {
        return target;
    }
    trimmed.split_ascii_whitespace().next().unwrap_or_default()
}

fn is_local_target(target: &str) -> bool {
    !target.is_empty()
        && !target.starts_with('#')
        && !target.starts_with("//")
        && !target.split_once(':').is_some_and(|(scheme, _)| {
            scheme.starts_with(|character: char| character.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "+-.".contains(character))
        })
}

fn unresolved_link(root: &Path, document: &Path, target: &str) -> Option<String> {
    let path = target.split('#').next().unwrap_or(target);
    let path = path.split('?').next().unwrap_or(path);
    if path.is_empty() {
        return None;
    }
    let joined = document.parent()?.join(path);
    if !joined.exists() {
        return Some(format!("local link `{target}` does not resolve"));
    }
    let canonical_root = root.canonicalize().ok()?;
    let canonical_target = joined.canonicalize().ok()?;
    if !canonical_target.starts_with(canonical_root) {
        return Some(format!("local link `{target}` escapes the repository"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Link, local_links, unresolved_link};
    use std::path::Path;

    #[test]
    fn local_links_ignore_external_anchors_and_fenced_examples() {
        let source = "[local](../README.md) [web](https://example.com) [part](#part)\n\
                      [guide]: ../CONTRIBUTING.md \"title\"\n\
                      ```md\n[example](missing.md)\n```\n";
        assert_eq!(
            local_links(source),
            vec![
                Link {
                    line: 1,
                    target: "../README.md".to_owned(),
                },
                Link {
                    line: 2,
                    target: "../CONTRIBUTING.md".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn a_missing_target_is_rejected_but_a_fragment_on_an_existing_file_is_not() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let document = root.join("docs").join("design").join("api-spine.md");
        assert!(unresolved_link(root, &document, "../../missing.md").is_some());
        assert!(unresolved_link(root, &document, "../../README.md#usage").is_none());
    }
}
