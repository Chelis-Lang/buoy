//! Shell transport for external objects and core navigation links.

use buoy_core::{
    AtomRevisionRef, CanonicalValue, EncodingError, ScopeId, SemanticDigest, SourceItemKind,
    StableId, semantic_digest,
};
pub use buoy_core::{ExternalTraceLink, ExternalTraceRelation, ExternalTraceTarget};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Stable failure classes for external traceability data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceabilityError {
    /// An external object ID is empty or noncanonical.
    ExternalObjectId,
    /// A report row is malformed or inconsistent.
    Report,
    /// A canonical set contains a duplicate value.
    Duplicate,
    /// Canonical digest computation failed.
    Encoding,
}

impl TraceabilityError {
    /// Returns the stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExternalObjectId => "SPG-EXTERNAL-OBJECT-ID",
            Self::Report => "SPG-EXTERNAL-TRACE-REPORT",
            Self::Duplicate => "SPG-EXTERNAL-TRACE-DUPLICATE",
            Self::Encoding => "DET-ENC-EXTERNAL-TRACE",
        }
    }
}

/// A canonical object ID in an external authority system.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ExternalObjectId(String);

impl ExternalObjectId {
    /// Parses a lower-case path ID.
    ///
    /// # Errors
    ///
    /// Returns `ExternalObjectId` for an empty component or invalid byte.
    pub fn new(input: &str) -> Result<Self, TraceabilityError> {
        let valid = !input.is_empty()
            && input.len() <= 512
            && input.split('/').all(|component| {
                !component.is_empty()
                    && component.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            });
        if valid {
            Ok(Self(input.to_owned()))
        } else {
            Err(TraceabilityError::ExternalObjectId)
        }
    }

    /// Returns the canonical external object ID.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An exact object reference in an external authority system.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExternalObjectRef {
    system_id: ScopeId,
    kind: SourceItemKind,
    id: ExternalObjectId,
    revision: SemanticDigest,
    digest: SemanticDigest,
}

impl ExternalObjectRef {
    /// Constructs one canonical external object reference.
    ///
    /// # Errors
    ///
    /// Returns `Encoding` if canonical digest computation fails.
    pub fn new(
        system_id: ScopeId,
        kind: SourceItemKind,
        id: ExternalObjectId,
        revision: SemanticDigest,
    ) -> Result<Self, TraceabilityError> {
        Self::new_internal(system_id, kind, id, revision, false)
    }

    fn new_internal(
        system_id: ScopeId,
        kind: SourceItemKind,
        id: ExternalObjectId,
        revision: SemanticDigest,
        inject_encoding_error: bool,
    ) -> Result<Self, TraceabilityError> {
        let encoded = if inject_encoding_error {
            Err(EncodingError::UnavailableAlgorithm)
        } else {
            semantic_digest(
                "external-object-ref",
                vec![
                    (1, CanonicalValue::Text(system_id.as_str().to_owned())),
                    (2, CanonicalValue::Text(kind.as_str().to_owned())),
                    (3, CanonicalValue::Text(id.as_str().to_owned())),
                    (4, CanonicalValue::Digest(revision)),
                ],
            )
        };
        let digest = encoded.map_err(|_| TraceabilityError::Encoding)?;
        Ok(Self {
            system_id,
            kind,
            id,
            revision,
            digest,
        })
    }

    /// Returns the external authority system ID.
    #[must_use]
    pub const fn system_id(&self) -> &ScopeId {
        &self.system_id
    }

    /// Returns the versioned external object kind.
    #[must_use]
    pub const fn kind(&self) -> &SourceItemKind {
        &self.kind
    }

    /// Returns the canonical external object ID.
    #[must_use]
    pub const fn id(&self) -> &ExternalObjectId {
        &self.id
    }

    /// Returns the exact external object revision.
    #[must_use]
    pub const fn revision(&self) -> SemanticDigest {
        self.revision
    }

    /// Returns the complete external reference digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// A canonical set of external trace links with TSV transport support.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalTraceReport {
    links: Vec<ExternalTraceLink>,
}

impl ExternalTraceReport {
    /// Constructs one canonical link report.
    ///
    /// # Errors
    ///
    /// Returns `Duplicate` if two links have the same ID or digest.
    pub fn new(mut links: Vec<ExternalTraceLink>) -> Result<Self, TraceabilityError> {
        links.sort_by(|left, right| {
            (left.meta().id().as_str(), left.meta().digest())
                .cmp(&(right.meta().id().as_str(), right.meta().digest()))
        });
        let ids = links
            .iter()
            .map(|link| link.meta().id().as_str())
            .collect::<BTreeSet<_>>();
        let digests = links
            .iter()
            .map(|link| link.meta().digest())
            .collect::<BTreeSet<_>>();
        let duplicate_id = ids.len() != links.len();
        let duplicate_digest = digests.len() != links.len();
        if duplicate_id | duplicate_digest {
            return Err(TraceabilityError::Duplicate);
        }
        Ok(Self { links })
    }

    /// Returns the canonical link rows.
    #[must_use]
    pub fn links(&self) -> &[ExternalTraceLink] {
        &self.links
    }

    /// Encodes the report as canonical schema-v1 TSV.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn encode_tsv(&self) -> String {
        let mut output = String::from("schema\tlink_id\tatoms\trelation\ttarget\tlink_digest\n");
        for link in &self.links {
            let atoms = link
                .atoms()
                .iter()
                .map(|atom| format!("{}@{}", atom.id().as_str(), atom.revision()))
                .collect::<Vec<_>>()
                .join(";");
            writeln!(
                output,
                "external-trace/v1\t{}\t{}\t{}\t{}\t{}",
                link.meta().id().as_str(),
                atoms,
                link.relation().as_str(),
                link.target().as_str(),
                link.meta().digest(),
            )
            .expect("writing to a String cannot fail");
        }
        output
    }

    /// Renders deterministic navigation text without provider state.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn render_human(&self) -> String {
        let mut output = String::from("External navigation links\n");
        for link in &self.links {
            writeln!(
                output,
                "- {} [{}] {}",
                link.meta().id().as_str(),
                link.relation().as_str(),
                link.target().as_str(),
            )
            .expect("writing to a String cannot fail");
        }
        output
    }

    /// Parses canonical schema-v1 TSV and verifies every link digest.
    ///
    /// # Errors
    ///
    /// Returns `Report` for a malformed row or digest mismatch.
    pub fn parse_tsv(input: &str) -> Result<Self, TraceabilityError> {
        const HEADER: &str = "schema\tlink_id\tatoms\trelation\ttarget\tlink_digest";
        let mut records = input.lines();
        if records.next() != Some(HEADER) {
            return Err(TraceabilityError::Report);
        }
        let mut links = Vec::new();
        for line in records {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 6 || fields[0] != "external-trace/v1" {
                return Err(TraceabilityError::Report);
            }
            let id = StableId::new(fields[1]).map_err(|_| TraceabilityError::Report)?;
            let atoms = parse_atoms(fields[2])?;
            let relation =
                ExternalTraceRelation::parse(fields[3]).map_err(|_| TraceabilityError::Report)?;
            let target =
                ExternalTraceTarget::new(fields[4]).map_err(|_| TraceabilityError::Report)?;
            let expected = SemanticDigest::parse_canonical(fields[5])
                .map_err(|_| TraceabilityError::Report)?;
            let link = ExternalTraceLink::new(id, atoms, relation, target)
                .map_err(|_| TraceabilityError::Report)?;
            if link.meta().digest() != expected {
                return Err(TraceabilityError::Report);
            }
            links.push(link);
        }
        Self::new(links)
    }
}

fn parse_atoms(input: &str) -> Result<Vec<AtomRevisionRef>, TraceabilityError> {
    if input.is_empty() {
        return Err(TraceabilityError::Report);
    }
    input
        .split(';')
        .map(|binding| {
            let (id, digest) = binding.split_once('@').ok_or(TraceabilityError::Report)?;
            Ok(AtomRevisionRef::new(
                StableId::new(id).map_err(|_| TraceabilityError::Report)?,
                SemanticDigest::parse_canonical(digest).map_err(|_| TraceabilityError::Report)?,
            ))
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn link() -> ExternalTraceLink {
        ExternalTraceLink::new(
            StableId::new("TRACE-LINK").expect("link ID"),
            vec![AtomRevisionRef::new(
                StableId::new("ATOM-TRACE").expect("atom ID"),
                SemanticDigest::xxh3_128([2; 16]),
            )],
            ExternalTraceRelation::Tracks,
            ExternalTraceTarget::new("https://example.com/issues/42").expect("target"),
        )
        .expect("trace link")
    }

    #[test]
    fn external_trace_report_round_trips_and_detects_stale_atoms() {
        let link = link();
        assert_eq!(link.stale_atom_ids(&BTreeMap::new()).len(), 1);
        let current = BTreeMap::from([(
            StableId::new("ATOM-TRACE").expect("atom ID"),
            SemanticDigest::xxh3_128([2; 16]),
        )]);
        assert!(link.stale_atom_ids(&current).is_empty());
        let report = ExternalTraceReport::new(vec![link]).expect("trace report");
        let parsed = ExternalTraceReport::parse_tsv(&report.encode_tsv()).expect("parsed report");
        assert_eq!(report, parsed);
    }

    #[test]
    fn external_trace_link_rejects_malformed_data() {
        assert_eq!(
            ExternalObjectRef::new_internal(
                ScopeId::new("external").expect("scope"),
                SourceItemKind::new("openspec/requirement/v1").expect("item kind"),
                ExternalObjectId::new("issue-1").expect("object ID"),
                SemanticDigest::xxh3_128([0; 16]),
                true,
            ),
            Err(TraceabilityError::Encoding)
        );
        assert_eq!(
            ExternalTraceReport::parse_tsv("bad\n").unwrap_err().code(),
            "SPG-EXTERNAL-TRACE-REPORT"
        );
        for invalid in ["", &"a".repeat(513), "issue//42", "Bad ID"] {
            assert!(ExternalObjectId::new(invalid).is_err());
        }
        assert!(ExternalTraceRelation::parse("closes").is_err());
        assert!(ExternalTraceTarget::new("http://example.com").is_err());
        assert!(ExternalTraceTarget::new("https://user@example.com/x").is_err());
        assert!(ExternalTraceTarget::new("https://example.com/x#fragment").is_err());
        assert!(ExternalTraceTarget::new("https://EXAMPLE.com/x").is_err());
        assert!(ExternalTraceTarget::new("https://bad_host/x").is_err());
        assert!(ExternalTraceTarget::new("https://-example.com/x").is_err());
        assert!(ExternalTraceTarget::new("urn:buoy:document:v1").is_ok());
        assert!(ExternalTraceTarget::new("urn:buoy:v1:document").is_err());
    }
}
