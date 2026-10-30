#![doc = "Closed portable glob parsing, matching, and language-overlap contracts."]

use buoy::core::RepositoryPath;
use buoy::glob::{GlobError, PortableGlob};
use buoy::repository::{RepositoryConfig, RepositoryError};

fn path(value: &str) -> Option<RepositoryPath> {
    RepositoryPath::new(value).ok()
}

#[test]
fn portable_glob_matches_only_the_closed_versioned_grammar() {
    let cases = [
        ("**/*.md", "spec/requirements.md", true),
        ("**/*.md", "requirements.md", true),
        ("spec/?eadme.*", "spec/readme.md", true),
        ("spec/?eadme.*", "spec/README.md", false),
        ("src/**/test?.rs", "src/a/b/test1.rs", true),
        ("src/**/test?.rs", "src/test12.rs", false),
        ("*", "one", true),
        ("*", "one/two", false),
        ("*?", "a", true),
        ("a?", "a", false),
    ];
    for (pattern, input, expected) in cases {
        let pattern = PortableGlob::parse(pattern);
        let input = path(input);
        assert!(pattern.is_ok() && input.is_some());
        let (Ok(pattern), Some(input)) = (pattern, input) else {
            continue;
        };
        assert_eq!(pattern.matches(&input), expected, "{pattern:?} {input:?}");
    }
}

#[test]
fn portable_glob_rejects_host_extensions_and_noncanonical_forms() {
    for (pattern, expected) in [
        ("", GlobError::NotRelative),
        ("/absolute", GlobError::NotRelative),
        ("C:/drive", GlobError::NotRelative),
        ("trailing/", GlobError::NotRelative),
        ("../escape", GlobError::Segment),
        ("a//b", GlobError::Segment),
        ("a/**x/b", GlobError::Syntax),
        ("a/[ab].md", GlobError::Syntax),
        ("a\\b", GlobError::Syntax),
        ("spec/e\u{301}.md", GlobError::Syntax),
        ("spec/control\n.md", GlobError::Syntax),
    ] {
        assert_eq!(PortableGlob::parse(pattern), Err(expected));
    }

    let config = br#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]
[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["[host-class].md"]
exclude = []
authority_expectation = "optional"
"#;
    assert_eq!(
        RepositoryConfig::parse(config),
        Err(RepositoryError::GlobSyntax)
    );
    assert_eq!(GlobError::NotRelative.code(), "DET-PATH-GLOB-NOT-RELATIVE");
    assert_eq!(GlobError::Syntax.code(), "DET-PATH-GLOB-SYNTAX");
    assert_eq!(GlobError::Segment.code(), "DET-PATH-GLOB-SEGMENT");
    assert_eq!(
        PortableGlob::parse("**/*.md").map(|pattern| pattern.as_str().to_owned()),
        Ok(String::from("**/*.md"))
    );
}

#[test]
fn portable_glob_language_overlap_is_exact_for_recursive_and_segment_wildcards() {
    for (left, right, expected) in [
        ("**/*.md", "spec/*.md", true),
        ("src/**/test?.rs", "src/test1.rs", true),
        ("src/**/test?.rs", "tests/**", false),
        ("a*b", "a?b", true),
        ("literal", "other", false),
        ("**", "a/b/c", true),
        ("a/**/z", "a/b/**", true),
        ("a/**/z", "b/**/z", false),
        ("*.md", "*.rs", false),
        ("?", "*", true),
        ("*", "*", true),
    ] {
        let left = PortableGlob::parse(left);
        let right = PortableGlob::parse(right);
        assert!(left.is_ok() && right.is_ok());
        let (Ok(left), Ok(right)) = (left, right) else {
            continue;
        };
        assert_eq!(left.overlaps(&right), expected, "{left:?} {right:?}");
        assert_eq!(right.overlaps(&left), expected, "symmetry");
    }
}
