use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

use crate::language::Language;
use crate::model::{Analysis, Call, CallKind, Function};

#[derive(Debug, Clone)]
struct SourceFile {
    path: PathBuf,
    display_path: String,
    language: Language,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FunctionKey {
    file: String,
    line: usize,
    name: String,
}

#[derive(Debug, Default)]
struct ParsedFile {
    definitions: Vec<FunctionKey>,
    call_sites: Vec<CallSite>,
}

#[derive(Debug, Clone)]
struct CallSite {
    caller: FunctionKey,
    callee_name: String,
    file: String,
    line: usize,
    kind: CallKind,
}

/// Analyzer behavior toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisOptions {
    pub include_tests: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            include_tests: true,
        }
    }
}

impl AnalysisOptions {
    /// Exclude common test files and Rust `#[cfg(test)]` modules.
    pub fn without_tests() -> Self {
        Self {
            include_tests: false,
        }
    }
}

/// Analyze a source file or directory and resolve calls between discovered functions.
pub fn analyze_path(input: impl AsRef<Path>, language: Option<Language>) -> Result<Analysis> {
    analyze_path_with_options(input, language, AnalysisOptions::default())
}

/// Analyze source with explicit options.
pub fn analyze_path_with_options(
    input: impl AsRef<Path>,
    language: Option<Language>,
    options: AnalysisOptions,
) -> Result<Analysis> {
    let input = input.as_ref();
    let files = discover_files(input, language, options)?;
    let parsed_files = files
        .par_iter()
        .map(|file| parse_file(file, options))
        .collect::<Result<Vec<_>>>()?;

    let mut definitions: Vec<_> = parsed_files
        .iter()
        .flat_map(|file| file.definitions.iter().cloned())
        .collect();

    definitions.sort();

    let functions: Vec<Function> = definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| Function {
            id: format!("f{index}"),
            name: definition.name.clone(),
            file: definition.file.clone(),
            line: definition.line,
        })
        .collect();

    let mut id_by_definition = BTreeMap::new();
    let mut id_by_name = BTreeMap::new();
    for function in &functions {
        let key = FunctionKey {
            file: function.file.clone(),
            line: function.line,
            name: function.name.clone(),
        };
        id_by_definition.insert(key, function.id.clone());
        id_by_name
            .entry(function.name.clone())
            .or_insert_with(|| function.id.clone());
    }

    let mut calls: Vec<_> = parsed_files
        .par_iter()
        .flat_map_iter(|file| {
            file.call_sites.iter().filter_map(|site| {
                let caller = id_by_definition.get(&site.caller)?;
                let callee = id_by_name.get(&site.callee_name)?;
                Some(Call {
                    caller: caller.clone(),
                    callee: callee.clone(),
                    file: site.file.clone(),
                    line: site.line,
                    kind: site.kind,
                })
            })
        })
        .collect();

    calls.sort_by(|left, right| {
        left.caller
            .cmp(&right.caller)
            .then(left.callee.cmp(&right.callee))
            .then(left.file.cmp(&right.file))
            .then(left.line.cmp(&right.line))
    });

    Ok(Analysis { functions, calls })
}

fn discover_files(
    input: &Path,
    language: Option<Language>,
    options: AnalysisOptions,
) -> Result<Vec<SourceFile>> {
    if !input.exists() {
        bail!("input path does not exist: {}", input.display());
    }

    let root = if input.is_file() {
        input.parent().unwrap_or_else(|| Path::new(""))
    } else {
        input
    };

    let mut files = Vec::new();
    if input.is_file() {
        maybe_push_source_file(input, root, language, options, &mut files)?;
    } else {
        for entry in WalkDir::new(input).sort_by_file_name() {
            let entry = entry.with_context(|| format!("failed to walk {}", input.display()))?;
            if should_skip_entry(entry.path(), root, options) {
                continue;
            }
            if entry.file_type().is_file() {
                maybe_push_source_file(entry.path(), root, language, options, &mut files)?;
            }
        }
    }

    files.sort_by(|left, right| left.display_path.cmp(&right.display_path));
    Ok(files)
}

fn maybe_push_source_file(
    path: &Path,
    root: &Path,
    requested_language: Option<Language>,
    options: AnalysisOptions,
    files: &mut Vec<SourceFile>,
) -> Result<()> {
    let Some(detected_language) = Language::from_extension(path) else {
        return Ok(());
    };

    if requested_language.is_some_and(|language| language != detected_language) {
        return Ok(());
    }

    let display_path = display_path(path, root)?;
    if !options.include_tests && is_test_file(&display_path, detected_language) {
        return Ok(());
    }

    files.push(SourceFile {
        path: path.to_path_buf(),
        display_path,
        language: detected_language,
    });
    Ok(())
}

fn should_skip_entry(path: &Path, root: &Path, options: AnalysisOptions) -> bool {
    if options.include_tests {
        return false;
    }

    let Ok(display_path) = display_path(path, root) else {
        return false;
    };

    display_path
        .split('/')
        .any(|component| matches!(component, ".git" | "target" | "tests"))
}

fn is_test_file(display_path: &str, language: Language) -> bool {
    if display_path
        .split('/')
        .any(|component| component == "tests")
    {
        return true;
    }

    match language {
        Language::Go => display_path.ends_with("_test.go"),
        Language::Rust => {
            display_path.ends_with("_test.rs")
                || display_path.ends_with("_tests.rs")
                || display_path.ends_with("/test.rs")
                || display_path.ends_with("/tests.rs")
        }
    }
}

fn display_path(path: &Path, root: &Path) -> Result<String> {
    let display_path = path.strip_prefix(root).unwrap_or(path);
    let value = display_path
        .to_str()
        .with_context(|| format!("path is not valid UTF-8: {}", path.display()))?;
    Ok(value.replace('\\', "/"))
}

fn parse_source(language: Language, source: &[u8], path: &Path) -> Result<tree_sitter::Tree> {
    let mut parser = Parser::new();
    let ts_language: tree_sitter::Language = match language {
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
    };
    parser
        .set_language(&ts_language)
        .with_context(|| format!("failed to load {language} grammar"))?;

    let tree = parser
        .parse(source, None)
        .with_context(|| format!("tree-sitter failed to parse {}", path.display()))?;
    if tree.root_node().has_error() {
        bail!("syntax errors found in {}", path.display());
    }
    Ok(tree)
}

fn parse_file(file: &SourceFile, options: AnalysisOptions) -> Result<ParsedFile> {
    let source =
        fs::read(&file.path).with_context(|| format!("failed to read {}", file.path.display()))?;
    let tree = parse_source(file.language, &source, &file.path)?;
    let mut parsed = ParsedFile::default();
    collect_file_symbols(
        tree.root_node(),
        &source,
        file.language,
        &file.display_path,
        options,
        None,
        &mut parsed,
    );
    Ok(parsed)
}

fn collect_file_symbols(
    node: Node<'_>,
    source: &[u8],
    language: Language,
    file: &str,
    options: AnalysisOptions,
    current_function: Option<&FunctionKey>,
    parsed: &mut ParsedFile,
) {
    if should_skip_node(language, node, source, options) {
        return;
    }

    let active_function_key;
    let active_function = if is_function_definition(language, node)
        && let Some(name) = definition_name(node, source)
    {
        active_function_key = FunctionKey {
            file: file.to_string(),
            line: line_number(node),
            name,
        };
        parsed.definitions.push(active_function_key.clone());
        Some(&active_function_key)
    } else {
        current_function
    };

    if node.kind() == "call_expression" {
        let kind = call_kind(node);
        if let (Some(caller), Some(callee_name)) = (active_function, call_simple_name(node, source))
        {
            parsed.call_sites.push(CallSite {
                caller: caller.clone(),
                callee_name,
                file: file.to_string(),
                line: line_number(node),
                kind,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_file_symbols(
            child,
            source,
            language,
            file,
            options,
            active_function,
            parsed,
        );
    }
}

fn is_function_definition(language: Language, node: Node<'_>) -> bool {
    match language {
        Language::Go => matches!(node.kind(), "function_declaration" | "method_declaration"),
        Language::Rust => node.kind() == "function_item",
    }
}

fn should_skip_node(
    language: Language,
    node: Node<'_>,
    source: &[u8],
    options: AnalysisOptions,
) -> bool {
    !options.include_tests && language == Language::Rust && has_cfg_test_attribute(node, source)
}

fn has_cfg_test_attribute(node: Node<'_>, source: &[u8]) -> bool {
    if node_has_cfg_test_attribute(node, source) {
        return true;
    }

    let mut previous = node.prev_named_sibling();
    while let Some(sibling) = previous {
        if sibling.end_position().row + 1 < node.start_position().row {
            break;
        }
        if sibling.kind() != "attribute_item" {
            break;
        }
        if node_text(sibling, source).is_some_and(|text| text.contains("cfg(test)")) {
            return true;
        }
        previous = sibling.prev_named_sibling();
    }

    false
}

fn node_has_cfg_test_attribute(node: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "attribute_item"
            && node_text(child, source).is_some_and(|text| text.contains("cfg(test)"))
        {
            return true;
        }
        if child.kind() != "attribute_item" {
            break;
        }
    }

    false
}

fn definition_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|name| node_text(name, source))
}

fn call_simple_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let function = node.child_by_field_name("function")?;
    terminal_name(function, source)
}

fn call_kind(node: Node<'_>) -> CallKind {
    node.child_by_field_name("function")
        .map(call_function_kind)
        .unwrap_or(CallKind::Unknown)
}

fn call_function_kind(node: Node<'_>) -> CallKind {
    match node.kind() {
        "identifier" => CallKind::Direct,
        "selector_expression" | "field_expression" => CallKind::Method,
        "scoped_identifier" => CallKind::Associated,
        "generic_function" => node
            .child_by_field_name("function")
            .map(generic_call_function_kind)
            .unwrap_or(CallKind::Unknown),
        _ => CallKind::Unknown,
    }
}

fn generic_call_function_kind(node: Node<'_>) -> CallKind {
    match node.kind() {
        "identifier" => CallKind::Direct,
        "scoped_identifier" => CallKind::Associated,
        _ => CallKind::Unknown,
    }
}

fn terminal_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => node_text(node, source),
        "generic_function" => node
            .child_by_field_name("function")
            .and_then(|function| terminal_name(function, source)),
        "scoped_identifier" => node
            .child_by_field_name("name")
            .and_then(|name| terminal_name(name, source))
            .or_else(|| last_named_child_name(node, source)),
        "selector_expression" | "field_expression" => node
            .child_by_field_name("field")
            .and_then(|field| terminal_name(field, source))
            .or_else(|| last_named_child_name(node, source)),
        _ => {
            if node.named_child_count() == 1 {
                node.named_child(0)
                    .and_then(|child| terminal_name(child, source))
            } else {
                None
            }
        }
    }
}

fn last_named_child_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let count = node.named_child_count();
    if count == 0 {
        return None;
    }
    node.named_child((count - 1) as u32)
        .and_then(|child| terminal_name(child, source))
}

fn node_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(str::to_string)
}

fn line_number(node: Node<'_>) -> usize {
    node.start_position().row + 1
}

#[cfg(test)]
mod tests {
    use super::{Language, display_path};
    use std::path::Path;

    #[test]
    fn display_paths_use_forward_slashes() {
        assert_eq!(
            display_path(Path::new("fixtures/go/main.go"), Path::new("fixtures")).unwrap(),
            "go/main.go"
        );
    }

    #[test]
    fn language_filter_ignores_mismatched_files() {
        assert!(
            Language::from_extension("main.go").is_some_and(|language| language != Language::Rust)
        );
    }
}
