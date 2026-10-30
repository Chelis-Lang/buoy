//! Pinned vendored tree-sitter grammar binding for the Chelis Surf dialect.
//!
//! The vendored sources under `vendor/` are byte-exact copies of the
//! generated grammar artifacts from one exact Chelis revision. This crate
//! is the workspace's only sanctioned FFI boundary; the two audited unsafe
//! expressions below carry item-level allows and everything else stays
//! denied.

use tree_sitter::{Language, Node, Parser, ffi::TSLanguage};

/// Exact Chelis source revision of the vendored grammar artifacts.
pub const GRAMMAR_SOURCE_REVISION: &str = "fc3232ffe085d15b213bb7457b69a22063a200b9";
/// Chelis release tag that names the pinned revision.
pub const GRAMMAR_SOURCE_TAG: &str = "v0.18.5";

#[allow(unsafe_code)]
unsafe extern "C" {
    fn tree_sitter_chelis_surf() -> *const TSLanguage;
}

/// Returns the pinned Surf grammar as a tree-sitter language value.
#[must_use]
pub fn language() -> Language {
    #[allow(unsafe_code)]
    unsafe {
        Language::from_raw(tree_sitter_chelis_surf())
    }
}

/// One parsed top-level Surf declaration node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfDeclaration {
    /// Inclusive byte start of the declaration node.
    pub start: u64,
    /// Exclusive byte end of the declaration node.
    pub end: u64,
    /// Grammar kind of the inner declaration form.
    pub kind: String,
}

/// One defective source region the pinned grammar rejects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseDefect {
    /// Inclusive byte start of the defective region.
    pub start: u64,
    /// Exclusive byte end of the defective region.
    pub end: u64,
}

/// Total parse outcome over one saved Surf source.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SurfParse {
    /// Top-level declaration nodes in source order.
    pub declarations: Vec<SurfDeclaration>,
    /// First defective region when the grammar rejects the source.
    pub defect: Option<ParseDefect>,
}

/// Grammar loading failure independent of the parsed source content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SitterError {
    /// The pinned runtime rejected the vendored grammar.
    Language,
    /// The parser returned no tree for a well-formed invocation.
    Parse,
}

fn inner_kind(declaration: Node<'_>) -> String {
    let mut cursor = declaration.walk();
    let inner = declaration
        .named_children(&mut cursor)
        .find(|child| child.kind() != "line_comment" && child.kind() != "block_comment");
    inner.map_or_else(
        || declaration.kind().to_owned(),
        |child| child.kind().to_owned(),
    )
}

fn first_defect(root: Node<'_>) -> Option<ParseDefect> {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            return Some(ParseDefect {
                start: node.start_byte() as u64,
                end: node.end_byte() as u64,
            });
        }
        if !node.has_error() {
            continue;
        }
        let mut cursor = node.walk();
        let mut children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        children.reverse();
        stack.extend(children);
    }
    None
}

/// Parses one saved Surf source with the pinned grammar.
///
/// The outcome is total over the input: every top-level declaration node
/// appears in source order, and a source the grammar rejects reports its
/// first defective region instead of failing silently.
///
/// # Errors
///
/// Returns a [`SitterError`] when the pinned runtime rejects the vendored
/// grammar or returns no tree. Both cases are independent of the parsed
/// source content.
pub fn parse(source: &str) -> Result<SurfParse, SitterError> {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .map_err(|_| SitterError::Language)?;
    let Some(tree) = parser.parse(source, None) else {
        return Err(SitterError::Parse);
    };
    let root = tree.root_node();
    let mut declarations = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "declaration" {
            declarations.push(SurfDeclaration {
                start: child.start_byte() as u64,
                end: child.end_byte() as u64,
                kind: inner_kind(child),
            });
        }
    }
    let defect = if root.has_error() {
        Some(first_defect(root).unwrap_or(ParseDefect {
            start: 0,
            end: source.len() as u64,
        }))
    } else {
        None
    };
    Ok(SurfParse {
        declarations,
        defect,
    })
}
