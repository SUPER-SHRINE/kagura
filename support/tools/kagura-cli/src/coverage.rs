#![forbid(unsafe_code)]

use conformance_runner::CaseResult;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use toml::Value;

#[derive(Debug)]
struct Diagnostic {
    source: Option<String>,
    message: String,
}

#[derive(Debug)]
struct Requirement {
    id: String,
    spec: String,
    cases: Vec<String>,
    source: String,
}

#[derive(Debug)]
struct CaseHeader {
    id: String,
    source: String,
}

/// Validate the complete logical coverage manifest and the combined CPU/Bus case suite.
///
/// An empty result means that validation succeeded. Every returned result is a suite-level
/// coverage error and is deliberately produced before the reference runtime is started.
pub fn validate(conformance_root: &Path) -> Vec<CaseResult> {
    let mut diagnostics = Vec::new();
    let mut cases = Vec::new();
    discover_cases(conformance_root, "cpu", &mut cases, &mut diagnostics);
    discover_cases(conformance_root, "bus", &mut cases, &mut diagnostics);

    let mut fragments = Vec::new();
    discover_fragments(conformance_root, &mut fragments, &mut diagnostics);

    let mut requirements = Vec::new();
    for (path, name) in fragments {
        parse_fragment(
            conformance_root,
            &path,
            name,
            &mut requirements,
            &mut diagnostics,
        );
    }

    validate_case_ids(&cases, &mut diagnostics);
    validate_requirements(conformance_root, &requirements, &cases, &mut diagnostics);

    diagnostics
        .into_iter()
        .map(|diagnostic| {
            CaseResult::error(
                "coverage",
                diagnostic
                    .source
                    .unwrap_or_else(|| relative_path(conformance_root, conformance_root)),
                diagnostic.message,
            )
        })
        .collect()
}

fn discover_cases(
    root: &Path,
    suite: &str,
    cases: &mut Vec<CaseHeader>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let directory = root.join("cases").join(suite);
    let mut files = Vec::new();
    if let Err(error) = collect_case_files(&directory, &mut files) {
        diagnostics.push(Diagnostic {
            source: Some(relative_path(root, &directory)),
            message: format!("cannot discover {suite} cases: {error}"),
        });
        return;
    }
    files.sort_by_key(|left| relative_path(root, left));
    for path in files {
        let source = relative_path(root, &path);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(Diagnostic {
                    source: Some(source),
                    message: format!("cannot read case TOML: {error}"),
                });
                continue;
            }
        };
        let value = match text.parse::<Value>() {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(Diagnostic {
                    source: Some(source),
                    message: format!("invalid case TOML: {error}"),
                });
                continue;
            }
        };
        let Some(table) = value.as_table() else {
            diagnostics.push(Diagnostic {
                source: Some(source),
                message: "case TOML top-level must be a table".into(),
            });
            continue;
        };
        let format = table.get("format").and_then(Value::as_str);
        let version = table.get("version").and_then(Value::as_integer);
        let id = table.get("id").and_then(Value::as_str);
        let expected_format = if suite == "cpu" {
            "kagura-conformance-case"
        } else {
            "kagura-bus-conformance-case"
        };
        let mut valid = true;
        if format != Some(expected_format) {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("invalid {suite} case format"),
            });
            valid = false;
        }
        if version != Some(1) {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("unsupported {suite} case version"),
            });
            valid = false;
        }
        let Some(id) = id.filter(|id| !id.is_empty()) else {
            diagnostics.push(Diagnostic {
                source: Some(source),
                message: format!("{suite} case requires a non-empty string id"),
            });
            continue;
        };
        if !valid_case_id(id) {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("invalid {suite} case identifier `{id}`"),
            });
            valid = false;
        }
        if valid {
            cases.push(CaseHeader {
                id: id.to_owned(),
                source,
            });
        }
    }
}

fn collect_case_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(root).map_err(|error| error.to_string())?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect_case_files(&path, files)?;
        } else if path.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some("case.toml")
        {
            files.push(path);
        }
    }
    Ok(())
}

fn discover_fragments(
    root: &Path,
    fragments: &mut Vec<(PathBuf, String)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let directory = root.join("coverage");
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(Diagnostic {
                source: Some(relative_path(root, &directory)),
                message: format!("cannot discover coverage fragments: {error}"),
            });
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                diagnostics.push(Diagnostic {
                    source: Some(relative_path(root, &directory)),
                    message: format!("cannot read coverage directory entry: {error}"),
                });
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
        else {
            diagnostics.push(Diagnostic {
                source: Some(relative_path(root, &path)),
                message: "coverage fragment filename must be valid UTF-8".into(),
            });
            continue;
        };
        if name.ends_with(".toml") {
            fragments.push((path, name));
        }
    }
    fragments.sort_by(|left, right| left.1.cmp(&right.1));
    if fragments.is_empty() {
        diagnostics.push(Diagnostic {
            source: Some(relative_path(root, &directory)),
            message: "coverage directory contains no .toml fragments".into(),
        });
    }
}

fn parse_fragment(
    root: &Path,
    path: &Path,
    _name: String,
    requirements: &mut Vec<Requirement>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let source = relative_path(root, path);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(Diagnostic {
                source: Some(source),
                message: format!("cannot read coverage fragment: {error}"),
            });
            return;
        }
    };
    let value = match text.parse::<Value>() {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(Diagnostic {
                source: Some(source),
                message: format!("invalid coverage TOML: {error}"),
            });
            return;
        }
    };
    let Some(table) = value.as_table() else {
        diagnostics.push(Diagnostic {
            source: Some(source),
            message: "coverage fragment top-level must be a table".into(),
        });
        return;
    };
    if table.len() != 1 || !table.contains_key("requirements") {
        diagnostics.push(Diagnostic {
            source: Some(source.clone()),
            message: "coverage fragment may contain only top-level requirements".into(),
        });
    }
    let Some(entries) = table.get("requirements").and_then(Value::as_array) else {
        diagnostics.push(Diagnostic {
            source: Some(source),
            message: "coverage fragment requires an array of requirements".into(),
        });
        return;
    };
    if entries.is_empty() {
        diagnostics.push(Diagnostic {
            source: Some(source),
            message: "coverage fragment requirements must not be empty".into(),
        });
        return;
    }
    for entry in entries {
        let Some(entry) = entry.as_table() else {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: "each requirement must be a table".into(),
            });
            continue;
        };
        let expected = ["id", "spec", "statement", "cases"];
        if entry.len() != expected.len() || expected.iter().any(|key| !entry.contains_key(*key)) {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: "requirement fields must be exactly id, spec, statement, and cases".into(),
            });
            continue;
        }
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: "requirement id must be a string".into(),
            });
            continue;
        };
        let Some(spec) = entry.get("spec").and_then(Value::as_str) else {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("requirement {id} spec must be a string"),
            });
            continue;
        };
        let Some(statement) = entry.get("statement").and_then(Value::as_str) else {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("requirement {id} statement must be a string"),
            });
            continue;
        };
        let Some(case_values) = entry.get("cases").and_then(Value::as_array) else {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("requirement {id} cases must be an array of strings"),
            });
            continue;
        };
        let mut case_ids = Vec::new();
        let mut valid = true;
        for case in case_values {
            let Some(case) = case.as_str() else {
                diagnostics.push(Diagnostic {
                    source: Some(source.clone()),
                    message: format!("requirement {id} cases must contain only strings"),
                });
                valid = false;
                continue;
            };
            case_ids.push(case.to_owned());
        }
        if id.is_empty() || !valid_requirement_id(id) {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("invalid requirement id `{id}`"),
            });
            valid = false;
        }
        if statement.trim().is_empty() {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("requirement {id} statement must not be empty"),
            });
            valid = false;
        }
        if case_ids.is_empty() {
            diagnostics.push(Diagnostic {
                source: Some(source.clone()),
                message: format!("requirement {id} cases must not be empty"),
            });
            valid = false;
        }
        let mut seen = BTreeSet::new();
        for case in &case_ids {
            if !seen.insert(case) {
                diagnostics.push(Diagnostic {
                    source: Some(source.clone()),
                    message: format!("requirement {id} contains duplicate case reference `{case}`"),
                });
                valid = false;
            }
        }
        if valid {
            requirements.push(Requirement {
                id: id.to_owned(),
                spec: spec.to_owned(),
                cases: case_ids,
                source: source.clone(),
            });
        }
    }
}

fn validate_case_ids(cases: &[CaseHeader], diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = BTreeMap::<&str, &str>::new();
    for case in cases {
        if let Some(previous) = seen.insert(&case.id, &case.source) {
            diagnostics.push(Diagnostic {
                source: Some(case.source.clone()),
                message: format!("duplicate case id `{}` (also in {previous})", case.id),
            });
        }
    }
}

fn validate_requirements(
    root: &Path,
    requirements: &[Requirement],
    cases: &[CaseHeader],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut requirement_ids = BTreeSet::new();
    let case_ids = cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut referenced = BTreeSet::new();
    for requirement in requirements {
        if !requirement_ids.insert(&requirement.id) {
            diagnostics.push(Diagnostic {
                source: Some(requirement.source.clone()),
                message: format!("duplicate requirement id `{}`", requirement.id),
            });
        }
        validate_spec_reference(root, requirement, diagnostics);
        for case in &requirement.cases {
            if !case_ids.contains(case.as_str()) {
                diagnostics.push(Diagnostic {
                    source: Some(requirement.source.clone()),
                    message: format!(
                        "requirement {} references missing case `{case}`",
                        requirement.id
                    ),
                });
            } else {
                referenced.insert(case.as_str());
            }
        }
    }
    for case in cases {
        if !referenced.contains(case.id.as_str()) {
            diagnostics.push(Diagnostic {
                source: Some(case.source.clone()),
                message: format!("case `{}` is not referenced by any requirement", case.id),
            });
        }
    }
}

fn validate_spec_reference(
    root: &Path,
    requirement: &Requirement,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut parts = requirement.spec.split('#');
    let Some(path_part) = parts.next() else {
        return;
    };
    let Some(anchor) = parts.next() else {
        diagnostics.push(Diagnostic {
            source: Some(requirement.source.clone()),
            message: format!("requirement {} spec must be path#anchor", requirement.id),
        });
        return;
    };
    if parts.next().is_some()
        || path_part.is_empty()
        || anchor.is_empty()
        || requirement.spec.contains('?')
        || path_part.contains('\\')
    {
        diagnostics.push(Diagnostic {
            source: Some(requirement.source.clone()),
            message: format!("invalid spec reference `{}`", requirement.spec),
        });
        return;
    }
    let relative = Path::new(path_part);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::CurDir
                    | Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
        || relative
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("md")
    {
        diagnostics.push(Diagnostic {
            source: Some(requirement.source.clone()),
            message: format!("invalid spec path `{path_part}`"),
        });
        return;
    }
    let spec_root = match root.parent() {
        Some(root) => root,
        None => {
            diagnostics.push(Diagnostic {
                source: Some(requirement.source.clone()),
                message: "conformance root has no specification parent".into(),
            });
            return;
        }
    };
    let canonical_spec_root = match fs::canonicalize(spec_root) {
        Ok(path) => path,
        Err(error) => {
            diagnostics.push(Diagnostic {
                source: Some(requirement.source.clone()),
                message: format!("cannot resolve specification root: {error}"),
            });
            return;
        }
    };
    let resolved = spec_root.join(relative);
    let canonical_root = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(error) => {
            diagnostics.push(Diagnostic {
                source: Some(requirement.source.clone()),
                message: format!("cannot resolve conformance root: {error}"),
            });
            return;
        }
    };
    let canonical_target = match fs::canonicalize(&resolved) {
        Ok(path) => path,
        Err(error) => {
            diagnostics.push(Diagnostic {
                source: Some(requirement.source.clone()),
                message: format!("spec path does not resolve to a file `{path_part}`: {error}"),
            });
            return;
        }
    };
    let Ok(_) = canonical_target.strip_prefix(&canonical_spec_root) else {
        diagnostics.push(Diagnostic {
            source: Some(requirement.source.clone()),
            message: format!("spec path escapes specification root: `{path_part}`"),
        });
        return;
    };
    if canonical_target.starts_with(&canonical_root)
        || !fs::symlink_metadata(&canonical_target)
            .map(|metadata| metadata.file_type().is_file())
            .unwrap_or(false)
        || canonical_target
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("md")
    {
        diagnostics.push(Diagnostic {
            source: Some(requirement.source.clone()),
            message: format!("spec path is not a normative Markdown file: `{path_part}`"),
        });
        return;
    }
    let text = match fs::read_to_string(&canonical_target) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(Diagnostic {
                source: Some(requirement.source.clone()),
                message: format!("cannot read spec reference `{path_part}`: {error}"),
            });
            return;
        }
    };
    let anchors = markdown_anchors(&text);
    let mut duplicate_anchors = BTreeSet::new();
    let mut seen_anchors = BTreeSet::new();
    for candidate in &anchors {
        if !seen_anchors.insert(candidate) {
            duplicate_anchors.insert(candidate);
        }
    }
    if !duplicate_anchors.is_empty() {
        diagnostics.push(Diagnostic {
            source: Some(requirement.source.clone()),
            message: format!(
                "spec document `{path_part}` generates duplicate heading anchors: {}",
                duplicate_anchors
                    .into_iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    if anchors
        .iter()
        .filter(|candidate| *candidate == anchor)
        .count()
        != 1
    {
        diagnostics.push(Diagnostic {
            source: Some(requirement.source.clone()),
            message: format!("spec anchor `{anchor}` is missing or ambiguous in `{path_part}`"),
        });
    }
}

fn markdown_anchors(text: &str) -> Vec<String> {
    let mut anchors = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start_matches(' ');
        if line.len() - trimmed.len() > 3 || !trimmed.starts_with('#') {
            continue;
        }
        let count = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if count == 0
            || (trimmed
                .as_bytes()
                .get(count)
                .is_some_and(|byte| !byte.is_ascii_whitespace()))
        {
            continue;
        }
        let heading = remove_atx_closing_sequence(trimmed[count..].trim());
        let anchor = heading
            .chars()
            .map(|character| {
                if character.is_whitespace() {
                    '-'
                } else {
                    character
                }
            })
            .filter_map(|character| {
                if character.is_ascii_alphabetic() {
                    Some(character.to_ascii_lowercase())
                } else if character.is_ascii_alphanumeric() || character == '-' || character == '_'
                {
                    Some(character)
                } else if character.is_ascii_punctuation() {
                    None
                } else {
                    Some(character)
                }
            })
            .collect();
        anchors.push(anchor);
    }
    anchors
}

fn remove_atx_closing_sequence(heading: &str) -> &str {
    let mut start = heading.len();
    for (index, character) in heading.char_indices().rev() {
        if character == '#' {
            start = index;
        } else {
            break;
        }
    }
    if start < heading.len()
        && heading[..start]
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
    {
        heading[..start].trim_end()
    } else {
        heading
    }
}

fn valid_requirement_id(id: &str) -> bool {
    let mut characters = id.chars();
    if !characters
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
    {
        return false;
    }
    if id.ends_with('-') || id.contains("--") {
        return false;
    }
    characters.all(|character| {
        character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-'
    })
}

fn valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id.is_ascii()
        && id
            .chars()
            .all(|character| !character.is_ascii_control() && !character.is_ascii_whitespace())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("kagura-coverage-{suffix}-{id}"))
            .join("conformance")
    }

    fn remove_fixture(root: &Path) {
        if let Some(container) = root.parent() {
            let _ = fs::remove_dir_all(container);
        }
    }

    fn write_fixture(root: &Path, coverage: &str, case_id: &str) {
        fs::create_dir_all(root.join("coverage")).unwrap();
        fs::create_dir_all(root.join("cases/cpu/example")).unwrap();
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::write(
            root.join("cases/cpu/example/case.toml"),
            format!("format = \"kagura-conformance-case\"\nversion = 1\nid = \"{case_id}\"\n"),
        )
        .unwrap();
        fs::write(root.join("coverage/a.toml"), coverage).unwrap();
        fs::write(
            root.parent().unwrap().join("cpu.md"),
            "# 1. Architectural State\n",
        )
        .unwrap();
    }

    fn has_message(root: &Path, needle: &str) -> bool {
        validate(root).iter().any(|result| {
            result
                .message
                .as_deref()
                .is_some_and(|message| message.contains(needle))
        })
    }

    #[test]
    fn valid_requirement_id_is_strict() {
        assert!(valid_requirement_id("CPU-ADD-1"));
        assert!(!valid_requirement_id("cpu-ADD"));
        assert!(!valid_requirement_id("CPU--ADD"));
        assert!(!valid_requirement_id("CPU-ADD-"));
    }

    #[test]
    fn markdown_anchor_algorithm_handles_atx_headings() {
        assert_eq!(
            markdown_anchors("## 3. ADD (arithmetic)\n"),
            vec!["3-add-arithmetic"]
        );
        assert_eq!(markdown_anchors("# Foo   bar ###\n"), vec!["foo---bar"]);
    }

    #[test]
    fn missing_case_is_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\ncases=[\"MISSING\"]\n",
            "PRESENT",
        );
        assert!(has_message(&root, "missing case"));
        remove_fixture(&root);
    }

    #[test]
    fn duplicate_case_reference_is_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\ncases=[\"PRESENT\", \"PRESENT\"]\n",
            "PRESENT",
        );
        assert!(validate(&root).iter().any(|result| {
            result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("duplicate case reference"))
        }));
        remove_fixture(&root);
    }

    #[test]
    fn unknown_fragment_field_is_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "extra = true\n[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
            "PRESENT",
        );
        assert!(has_message(&root, "only top-level requirements"));
        remove_fixture(&root);
    }

    #[test]
    fn duplicate_requirement_id_and_unknown_requirement_field_are_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
            "PRESENT",
        );
        fs::write(
            root.join("coverage/b.toml"),
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
        )
        .unwrap();
        fs::write(
            root.join("coverage/c.toml"),
            "[[requirements]]\nid=\"CPU-B\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\nextra=\"unknown\"\ncases=[\"PRESENT\"]\n",
        )
        .unwrap();
        assert!(has_message(&root, "duplicate requirement id"));
        assert!(has_message(&root, "exactly id, spec, statement, and cases"));
        remove_fixture(&root);
    }

    #[test]
    fn invalid_requirement_id_and_whitespace_statement_are_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"cpu-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"   \"\ncases=[\"PRESENT\"]\n",
            "PRESENT",
        );
        assert!(has_message(&root, "invalid requirement id"));
        assert!(has_message(&root, "statement must not be empty"));
        remove_fixture(&root);
    }

    #[test]
    fn uncovered_case_is_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\ncases=[\"MISSING\"]\n",
            "PRESENT",
        );
        assert!(has_message(&root, "not referenced by any requirement"));
        remove_fixture(&root);
    }

    #[test]
    fn invalid_spec_path_and_anchor_are_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"../cpu.md#missing\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
            "PRESENT",
        );
        assert!(has_message(&root, "invalid spec path"));
        remove_fixture(&root);
    }

    #[test]
    fn missing_spec_file_missing_anchor_and_duplicate_anchor_are_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"missing.md#missing\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
            "PRESENT",
        );
        assert!(has_message(&root, "spec path does not resolve"));
        fs::write(
            root.join("coverage/a.toml"),
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#missing\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
        )
        .unwrap();
        assert!(has_message(&root, "spec anchor `missing`"));
        fs::write(root.parent().unwrap().join("cpu.md"), "# Same\n# Same\n").unwrap();
        fs::write(
            root.join("coverage/a.toml"),
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#same\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
        )
        .unwrap();
        assert!(has_message(&root, "duplicate heading anchors"));
        remove_fixture(&root);
    }

    #[test]
    fn duplicate_case_id_and_invalid_case_header_are_reported() {
        let root = temp_root();
        write_fixture(
            &root,
            "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#1-architectural-state\"\nstatement=\"x\"\ncases=[\"PRESENT\"]\n",
            "PRESENT",
        );
        fs::create_dir_all(root.join("cases/bus/example")).unwrap();
        fs::write(
            root.join("cases/bus/example/case.toml"),
            "format=\"kagura-bus-conformance-case\"\nversion=1\nid=\"PRESENT\"\n",
        )
        .unwrap();
        assert!(has_message(&root, "duplicate case id"));
        fs::write(
            root.join("cases/bus/example/case.toml"),
            "format=\"wrong\"\nversion=9\nid=\"bad id\"\n",
        )
        .unwrap();
        assert!(has_message(&root, "invalid bus case format"));
        assert!(has_message(&root, "unsupported bus case version"));
        assert!(has_message(&root, "invalid bus case identifier"));
        remove_fixture(&root);
    }

    #[test]
    fn empty_fragment_directory_is_reported() {
        let root = temp_root();
        fs::create_dir_all(root.join("coverage")).unwrap();
        fs::create_dir_all(root.join("cases/cpu")).unwrap();
        fs::create_dir_all(root.join("cases/bus")).unwrap();
        let results = validate(&root);
        assert!(results.iter().any(|result| {
            result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("no .toml fragments"))
        }));
        remove_fixture(&root);
    }
}
