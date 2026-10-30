//! Closed locale-independent `spec-provenance-glob/v1` parser and matcher.

use buoy_core::RepositoryPath;
use std::collections::{BTreeSet, VecDeque};
use unicode_normalization::UnicodeNormalization;

/// One parsed closed portable glob pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableGlob {
    source: String,
    segments: Vec<Segment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Segment {
    Recursive,
    Pattern(Vec<Token>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Token {
    Literal(char),
    AnyOne,
    AnyMany,
}

/// Closed portable glob parse failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobError {
    /// Pattern is empty, absolute, directory-shaped, or drive-qualified.
    NotRelative,
    /// Pattern contains a forbidden, non-NFC, or unsupported grammar form.
    Syntax,
    /// Pattern contains an empty/current/parent path segment.
    Segment,
}

impl GlobError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotRelative => "DET-PATH-GLOB-NOT-RELATIVE",
            Self::Syntax => "DET-PATH-GLOB-SYNTAX",
            Self::Segment => "DET-PATH-GLOB-SEGMENT",
        }
    }
}

impl PortableGlob {
    /// Parses one exact `spec-provenance-glob/v1` pattern.
    ///
    /// `*` matches zero or more scalars within one path segment, `?` matches
    /// exactly one scalar within one segment, and `**` is accepted only as a
    /// complete segment and matches zero or more complete nonempty segments.
    /// Escapes, classes, braces, platform separators, and host extensions are
    /// deliberately unsupported.
    ///
    /// # Errors
    ///
    /// Returns a stable closed grammar error without consulting host glob or
    /// ignore semantics.
    pub fn parse(source: &str) -> Result<Self, GlobError> {
        if source.is_empty()
            || source.starts_with('/')
            || source.ends_with('/')
            || (source.len() >= 2 && source.as_bytes()[1] == b':')
        {
            return Err(GlobError::NotRelative);
        }
        if source.nfc().ne(source.chars())
            || source.contains('\\')
            || source.chars().any(|character| {
                character.is_ascii_control()
                    || matches!(character, '[' | ']' | '{' | '}' | '<' | '>' | '|' | '"')
            })
        {
            return Err(GlobError::Syntax);
        }
        let mut segments = Vec::new();
        for raw in source.split('/') {
            if raw.is_empty() || matches!(raw, "." | "..") {
                return Err(GlobError::Segment);
            }
            if raw == "**" {
                segments.push(Segment::Recursive);
                continue;
            }
            if raw.contains("**") {
                return Err(GlobError::Syntax);
            }
            let tokens = raw
                .chars()
                .map(|character| match character {
                    '*' => Token::AnyMany,
                    '?' => Token::AnyOne,
                    literal => Token::Literal(literal),
                })
                .collect::<Vec<_>>();
            segments.push(Segment::Pattern(tokens));
        }
        Ok(Self {
            source: source.to_owned(),
            segments,
        })
    }

    /// Exact canonical pattern source bytes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Matches one already-canonical repository-relative path.
    #[must_use]
    pub fn matches(&self, path: &RepositoryPath) -> bool {
        let segments = path.as_str().split('/').collect::<Vec<_>>();
        path_matches(&self.segments, &segments)
    }

    /// Returns whether two pattern languages share at least one canonical path.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        path_languages_overlap(&self.segments, &other.segments)
    }
}

fn path_matches(pattern: &[Segment], path: &[&str]) -> bool {
    let mut reachable = BTreeSet::from([(0_usize, 0_usize)]);
    let mut pending = VecDeque::from([(0_usize, 0_usize)]);
    while let Some((pattern_index, path_index)) = pending.pop_front() {
        if pattern_index == pattern.len() && path_index == path.len() {
            return true;
        }
        let Some(segment) = pattern.get(pattern_index) else {
            continue;
        };
        match segment {
            Segment::Recursive => {
                push_state(
                    (pattern_index + 1, path_index),
                    &mut reachable,
                    &mut pending,
                );
                if path_index < path.len() {
                    push_state(
                        (pattern_index, path_index + 1),
                        &mut reachable,
                        &mut pending,
                    );
                }
            }
            Segment::Pattern(tokens) => {
                if path
                    .get(path_index)
                    .is_some_and(|value| segment_matches(tokens, value))
                {
                    push_state(
                        (pattern_index + 1, path_index + 1),
                        &mut reachable,
                        &mut pending,
                    );
                }
            }
        }
    }
    false
}

fn segment_matches(pattern: &[Token], value: &str) -> bool {
    let characters = value.chars().collect::<Vec<_>>();
    let mut reachable = BTreeSet::from([(0_usize, 0_usize)]);
    let mut pending = VecDeque::from([(0_usize, 0_usize)]);
    while let Some((pattern_index, character_index)) = pending.pop_front() {
        if pattern_index == pattern.len() && character_index == characters.len() {
            return true;
        }
        let Some(token) = pattern.get(pattern_index) else {
            continue;
        };
        match token {
            Token::AnyMany => {
                push_state(
                    (pattern_index + 1, character_index),
                    &mut reachable,
                    &mut pending,
                );
                if character_index < characters.len() {
                    push_state(
                        (pattern_index, character_index + 1),
                        &mut reachable,
                        &mut pending,
                    );
                }
            }
            Token::AnyOne if character_index < characters.len() => push_state(
                (pattern_index + 1, character_index + 1),
                &mut reachable,
                &mut pending,
            ),
            Token::Literal(expected) if characters.get(character_index) == Some(expected) => {
                push_state(
                    (pattern_index + 1, character_index + 1),
                    &mut reachable,
                    &mut pending,
                );
            }
            Token::AnyOne | Token::Literal(_) => {}
        }
    }
    false
}

fn path_languages_overlap(left: &[Segment], right: &[Segment]) -> bool {
    let mut reached = BTreeSet::from([(0_usize, 0_usize)]);
    let mut pending = VecDeque::from([(0_usize, 0_usize)]);
    while let Some((left_index, right_index)) = pending.pop_front() {
        if left_index == left.len() && right_index == right.len() {
            return true;
        }
        let left_segment = left.get(left_index);
        let right_segment = right.get(right_index);
        if matches!(left_segment, Some(Segment::Recursive)) {
            push_state((left_index + 1, right_index), &mut reached, &mut pending);
        }
        if matches!(right_segment, Some(Segment::Recursive)) {
            push_state((left_index, right_index + 1), &mut reached, &mut pending);
        }
        match (left_segment, right_segment) {
            (Some(Segment::Pattern(left)), Some(Segment::Pattern(right)))
                if segment_languages_overlap(left, right) =>
            {
                push_state(
                    (left_index + 1, right_index + 1),
                    &mut reached,
                    &mut pending,
                );
            }
            (Some(Segment::Recursive), Some(Segment::Pattern(_))) => {
                push_state((left_index, right_index + 1), &mut reached, &mut pending);
            }
            (Some(Segment::Pattern(_)), Some(Segment::Recursive)) => {
                push_state((left_index + 1, right_index), &mut reached, &mut pending);
            }
            _ => {}
        }
    }
    false
}

fn segment_languages_overlap(left: &[Token], right: &[Token]) -> bool {
    let mut reached = BTreeSet::from([(0_usize, 0_usize, false)]);
    let mut pending = VecDeque::from([(0_usize, 0_usize, false)]);
    while let Some((left_index, right_index, consumed)) = pending.pop_front() {
        if left_index == left.len() && right_index == right.len() && consumed {
            return true;
        }
        if left.get(left_index) == Some(&Token::AnyMany) {
            push_state(
                (left_index + 1, right_index, consumed),
                &mut reached,
                &mut pending,
            );
        }
        if right.get(right_index) == Some(&Token::AnyMany) {
            push_state(
                (left_index, right_index + 1, consumed),
                &mut reached,
                &mut pending,
            );
        }
        let Some(left_token) = left.get(left_index) else {
            continue;
        };
        let Some(right_token) = right.get(right_index) else {
            continue;
        };
        if tokens_share_scalar(*left_token, *right_token) {
            let next_left = if *left_token == Token::AnyMany {
                left_index
            } else {
                left_index + 1
            };
            let next_right = if *right_token == Token::AnyMany {
                right_index
            } else {
                right_index + 1
            };
            push_state((next_left, next_right, true), &mut reached, &mut pending);
        }
    }
    false
}

const fn tokens_share_scalar(left: Token, right: Token) -> bool {
    match (left, right) {
        (Token::Literal(left), Token::Literal(right)) => left == right,
        _ => true,
    }
}

fn push_state<T: Ord + Copy>(state: T, reached: &mut BTreeSet<T>, pending: &mut VecDeque<T>) {
    if reached.insert(state) {
        pending.push_back(state);
    }
}
