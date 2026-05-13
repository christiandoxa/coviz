use coviz::{
    AnalysisOptions, Language, analyze_path, analyze_path_with_options, model::CallKind,
    render_dot, render_json,
};

#[test]
fn analyzes_go_fixture() {
    let analysis = analyze_path("tests/fixtures/go", Some(Language::Go)).unwrap();

    let names: Vec<_> = analysis
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(names, ["main", "serve", "worker", "ignored"]);

    let call_names: Vec<_> = analysis
        .calls
        .iter()
        .map(|call| {
            let caller = function_name(&analysis, &call.caller);
            let callee = function_name(&analysis, &call.callee);
            (caller, callee, call.line, call.kind)
        })
        .collect();
    assert_eq!(
        call_names,
        [
            ("main", "serve", 4, CallKind::Direct),
            ("main", "worker", 5, CallKind::Direct),
            ("serve", "worker", 9, CallKind::Direct),
        ]
    );
}

#[test]
fn analyzes_rust_fixture() {
    let analysis = analyze_path("tests/fixtures/rust", Some(Language::Rust)).unwrap();

    let names: Vec<_> = analysis
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(names, ["entry", "helper", "target", "target"]);

    let call_names: Vec<_> = analysis
        .calls
        .iter()
        .map(|call| {
            let caller = function_name(&analysis, &call.caller);
            let callee = function_name(&analysis, &call.callee);
            (caller, callee, call.line, call.kind)
        })
        .collect();
    assert_eq!(
        call_names,
        [
            ("entry", "helper", 2, CallKind::Direct),
            ("entry", "target", 3, CallKind::Associated),
            ("helper", "target", 7, CallKind::Direct),
        ]
    );
}

#[test]
fn renders_outputs_from_analysis() {
    let analysis = analyze_path("tests/fixtures/go", Some(Language::Go)).unwrap();

    let dot = render_dot(&analysis);
    assert!(dot.starts_with("digraph coviz {\n"));
    assert!(dot.contains("\"f0\" -> \"f1\""));

    let json = render_json(&analysis).unwrap();
    assert!(json.contains("\"functions\""));
    assert!(json.contains("\"calls\""));
}

#[test]
fn analyzes_mixed_directory_when_language_is_unspecified() {
    let analysis = analyze_path("tests/fixtures", None).unwrap();

    assert_eq!(analysis.functions.len(), 14);
    assert_eq!(analysis.calls.len(), 10);
}

#[test]
fn can_exclude_test_files_and_cfg_test_code() {
    let analysis = analyze_path_with_options(
        "tests/fixtures/filter-tests",
        None,
        AnalysisOptions::without_tests(),
    )
    .unwrap();

    let names: Vec<_> = analysis
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(names, ["main", "helper", "production", "shared"]);
    assert!(
        analysis
            .functions
            .iter()
            .all(|function| !function.file.ends_with("_test.go"))
    );
    assert!(
        analysis
            .functions
            .iter()
            .all(|function| function.name != "hidden_test")
    );
}

fn function_name<'a>(analysis: &'a coviz::Analysis, id: &str) -> &'a str {
    analysis
        .functions
        .iter()
        .find(|function| function.id == id)
        .map(|function| function.name.as_str())
        .unwrap()
}
