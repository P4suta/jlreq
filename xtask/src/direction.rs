// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Enforce the one-way module graph inside the unified library.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::shared::{self, Gate};

pub(crate) const GATE: Gate = Gate {
    name: "direction",
    purpose: "kumihan's private modules depend only on the declared earlier pipeline layers",
    reference: "ARCHITECTURE.md",
    run,
};

#[derive(Debug)]
struct Layer {
    name: &'static str,
    may_depend_on: &'static [&'static str],
}

const LAYERS: &[Layer] = &[
    Layer {
        name: "model",
        may_depend_on: &[],
    },
    Layer {
        name: "style",
        may_depend_on: &[],
    },
    Layer {
        name: "generated",
        may_depend_on: &["spec"],
    },
    Layer {
        name: "spec",
        may_depend_on: &["generated", "model"],
    },
    Layer {
        name: "normalize",
        may_depend_on: &["model", "spec"],
    },
    Layer {
        name: "construct",
        may_depend_on: &["model", "spec"],
    },
    Layer {
        name: "paragraph",
        may_depend_on: &["construct", "model", "spec", "style"],
    },
    Layer {
        name: "layout",
        may_depend_on: &["model"],
    },
    Layer {
        name: "pipeline",
        may_depend_on: &[
            "construct",
            "generated",
            "layout",
            "model",
            "normalize",
            "paragraph",
            "spec",
            "style",
        ],
    },
    Layer {
        name: "lib",
        may_depend_on: &[
            "construct",
            "generated",
            "layout",
            "model",
            "normalize",
            "paragraph",
            "pipeline",
            "spec",
            "style",
        ],
    },
];

fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    if let Some(first) = arguments.first() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the direction gate takes no arguments; got `{first}`"),
        ));
    }
    let root = shared::workspace_root()?
        .join("crates")
        .join("kumihan")
        .join("src");
    let mut sources = BTreeMap::new();
    for path in shared::rust_sources(&root)? {
        let module = module_of(&path, &root)?;
        let source = fs::read_to_string(&path)?;
        sources
            .entry(module)
            .or_insert_with(String::new)
            .push_str(&shared::code_only(&source));
    }
    let violations = check_graph(&sources);
    println!(
        "direction: examined {files} source module(s) across {layers} declared layer(s)",
        files = sources.len(),
        layers = LAYERS.len(),
    );
    Ok(violations)
}

fn module_of(path: &Path, root: &Path) -> io::Result<String> {
    let relative = path.strip_prefix(root).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is outside {}: {error}", path.display(), root.display()),
        )
    })?;
    let first = relative.components().next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "a Rust source has no relative name",
        )
    })?;
    let name = first.as_os_str().to_string_lossy();
    Ok(name.strip_suffix(".rs").unwrap_or(&name).to_owned())
}

fn check_graph(sources: &BTreeMap<String, String>) -> Vec<String> {
    let declared: BTreeSet<&str> = LAYERS.iter().map(|layer| layer.name).collect();
    let mut violations = Vec::new();
    for module in sources.keys() {
        if !declared.contains(module.as_str()) {
            violations.push(format!(
                "kumihan::{module} has no row in the module graph; update ARCHITECTURE.md and \
                 xtask/src/direction.rs together"
            ));
        }
    }
    for layer in LAYERS {
        let Some(source) = sources.get(layer.name) else {
            violations.push(format!(
                "the module graph names kumihan::{}, but no source module exists",
                layer.name
            ));
            continue;
        };
        if layer.name != "lib" && source.contains("use crate::{") {
            violations.push(format!(
                "kumihan::{} uses a crate-root grouped import; private dependencies must name \
                 their source module so this gate can check their direction",
                layer.name
            ));
        }
        for dependency in module_references(source, &declared) {
            if dependency != layer.name && !layer.may_depend_on.contains(&dependency.as_str()) {
                violations.push(format!(
                    "kumihan::{from} depends on kumihan::{dependency}; its row permits {allowed}",
                    from = layer.name,
                    allowed = if layer.may_depend_on.is_empty() {
                        "no private modules".to_owned()
                    } else {
                        layer.may_depend_on.join(", ")
                    }
                ));
            }
        }
    }
    violations
}

fn module_references(source: &str, declared: &BTreeSet<&str>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for tail in source.split("crate::").skip(1) {
        let name: String = tail
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if declared.contains(name.as_str()) {
            found.insert(name);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{check_graph, module_references};
    use std::collections::{BTreeMap, BTreeSet};

    fn empty_graph() -> BTreeMap<String, String> {
        super::LAYERS
            .iter()
            .map(|layer| (layer.name.to_owned(), String::new()))
            .collect()
    }

    #[test]
    fn a_reverse_reference_is_rejected() {
        let mut sources = empty_graph();
        sources.insert(
            "model".to_owned(),
            "use crate::pipeline::Composer;".to_owned(),
        );
        let found = check_graph(&sources);
        assert!(
            found
                .iter()
                .any(|message| message.contains("model depends on kumihan::pipeline")),
            "{found:#?}"
        );
    }

    #[test]
    fn only_declared_module_names_are_collected() {
        let declared = BTreeSet::from(["model", "spec"]);
        assert_eq!(
            module_references(
                "crate::model::Size; crate::Style; crate::spec::is_pair();",
                &declared
            ),
            BTreeSet::from(["model".to_owned(), "spec".to_owned()])
        );
    }
}
