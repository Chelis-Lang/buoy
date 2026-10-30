//! Parse contract for the pinned vendored Chelis Surf grammar.

use chelis_surf_sitter::{ParseDefect, parse};

const CANONICAL: &str = "module Demo.Mod\n\n-- plain comment\ndef scale(x: f32) -> f32 = {\n  y = x\n  y\n}\n\ntype Point = | Point { x: f32, y: f32 }\n";

#[test]
fn canonical_source_parses_without_defects() {
    let outcome = parse(CANONICAL).map_err(|error| format!("{error:?}"));
    let Ok(outcome) = outcome else {
        assert!(outcome.is_ok(), "grammar must load");
        return;
    };
    assert!(outcome.defect.is_none(), "canonical source has no defect");
    let kinds = outcome
        .declarations
        .iter()
        .map(|declaration| declaration.kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            "module_declaration",
            "function_definition",
            "type_definition"
        ],
    );
}

#[test]
fn declaration_spans_cover_exact_node_bytes() {
    let Ok(outcome) = parse(CANONICAL) else {
        unreachable!("grammar must load");
    };
    let function = &outcome.declarations[1];
    let start = usize::try_from(function.start).unwrap_or(usize::MAX);
    let end = usize::try_from(function.end).unwrap_or(usize::MAX);
    let text = &CANONICAL[start..end];
    assert!(
        text.starts_with("def scale"),
        "node starts at the def token"
    );
    assert!(text.ends_with('}'), "node ends at the closing brace");
}

#[test]
fn rejected_source_reports_the_first_defect() {
    for (source, start) in [
        ("module Demo.Mod\n\ndef broken( = {\n", 17),
        ("def value(x: f32) -> f32 =\n", 27),
    ] {
        let Ok(outcome) = parse(source) else {
            unreachable!("grammar must load");
        };
        assert_eq!(
            outcome.defect,
            Some(ParseDefect {
                start,
                end: source.len() as u64,
            })
        );
    }
}

#[test]
fn parse_is_deterministic() {
    let Ok(first) = parse(CANONICAL) else {
        unreachable!("grammar must load");
    };
    let Ok(second) = parse(CANONICAL) else {
        unreachable!("grammar must load");
    };
    assert_eq!(first, second);
}
