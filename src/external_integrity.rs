//! Field-specific opaque handling for ecosystem-prescribed integrity metadata.

/// Supported external metadata ecosystem.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExternalEcosystem {
    /// Cargo registry package checksum.
    Cargo,
    /// Nix fixed-output/SRI hash.
    Nix,
    /// Exact Git source revision.
    Git,
    /// Upstream release checksum/signature metadata.
    Upstream,
}

/// One opaque external integrity field that cannot become a semantic digest.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OpaqueExternalIntegrityField {
    ecosystem: ExternalEcosystem,
    field: String,
    value: String,
}

impl OpaqueExternalIntegrityField {
    /// External ecosystem that owns interpretation and verification.
    #[must_use]
    pub const fn ecosystem(&self) -> ExternalEcosystem {
        self.ecosystem
    }

    /// Exact allowed field name.
    #[must_use]
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Opaque field text retained without digest parsing or reinterpretation.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// External trust-boundary rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalIntegrityError {
    /// Input is not one exact supported ecosystem field shape.
    FieldSyntax,
    /// Input attempts a path-wide or substring-wide exception.
    BroadException,
    /// External metadata is parsed or promoted as a semantic identity.
    SemanticPromotion,
}

impl ExternalIntegrityError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::FieldSyntax => "XXH-EXTERNAL-FIELD-SYNTAX",
            Self::BroadException => "XXH-EXTERNAL-BROAD-EXCEPTION",
            Self::SemanticPromotion => "XXH-EXTERNAL-SEMANTIC-PROMOTION",
        }
    }
}

fn field(ecosystem: ExternalEcosystem, name: &str, value: &str) -> OpaqueExternalIntegrityField {
    OpaqueExternalIntegrityField {
        ecosystem,
        field: name.to_owned(),
        value: value.to_owned(),
    }
}

fn parse_cargo(source: &str) -> Result<Vec<OpaqueExternalIntegrityField>, ExternalIntegrityError> {
    let value =
        toml::from_str::<toml::Value>(source).map_err(|_| ExternalIntegrityError::FieldSyntax)?;
    let packages = value
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or(ExternalIntegrityError::FieldSyntax)?;
    let mut output = Vec::new();
    for package in packages {
        let table = package
            .as_table()
            .ok_or(ExternalIntegrityError::FieldSyntax)?;
        let source = table
            .get("source")
            .and_then(toml::Value::as_str)
            .ok_or(ExternalIntegrityError::FieldSyntax)?;
        let checksum = table
            .get("checksum")
            .and_then(toml::Value::as_str)
            .ok_or(ExternalIntegrityError::FieldSyntax)?;
        if !source.starts_with("registry+")
            || checksum.len() != 64
            || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ExternalIntegrityError::FieldSyntax);
        }
        output.push(field(ExternalEcosystem::Cargo, "checksum", checksum));
    }
    if output.is_empty() {
        return Err(ExternalIntegrityError::FieldSyntax);
    }
    Ok(output)
}

fn quoted_assignment<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    source.lines().find_map(|line| {
        let line = line.trim();
        let value = line.strip_prefix(name)?.trim_start();
        let value = value.strip_prefix('=')?.trim();
        value.strip_prefix('"')?.strip_suffix("\";")
    })
}

fn parse_nix(source: &str) -> Result<Vec<OpaqueExternalIntegrityField>, ExternalIntegrityError> {
    if !source.trim_start().starts_with("fetchurl {") {
        return Err(ExternalIntegrityError::FieldSyntax);
    }
    let hash = quoted_assignment(source, "hash").ok_or(ExternalIntegrityError::FieldSyntax)?;
    if !(hash.starts_with("sha256-") | hash.starts_with("sha512-")) {
        return Err(ExternalIntegrityError::FieldSyntax);
    }
    Ok(vec![field(ExternalEcosystem::Nix, "hash", hash)])
}

fn parse_git(source: &str) -> Result<Vec<OpaqueExternalIntegrityField>, ExternalIntegrityError> {
    let value =
        toml::from_str::<toml::Value>(source).map_err(|_| ExternalIntegrityError::FieldSyntax)?;
    let dependencies = value
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .ok_or(ExternalIntegrityError::FieldSyntax)?;
    let mut output = Vec::new();
    for dependency in dependencies.values() {
        let table = dependency
            .as_table()
            .ok_or(ExternalIntegrityError::FieldSyntax)?;
        let git = table
            .get("git")
            .and_then(toml::Value::as_str)
            .ok_or(ExternalIntegrityError::FieldSyntax)?;
        let revision = table
            .get("rev")
            .and_then(toml::Value::as_str)
            .ok_or(ExternalIntegrityError::FieldSyntax)?;
        if !(git.starts_with("https://") && revision.len() == 40) {
            return Err(ExternalIntegrityError::FieldSyntax);
        }
        output.push(field(ExternalEcosystem::Git, "rev", revision));
    }
    if output.is_empty() {
        return Err(ExternalIntegrityError::FieldSyntax);
    }
    Ok(output)
}

fn parse_upstream(
    source: &str,
) -> Result<Vec<OpaqueExternalIntegrityField>, ExternalIntegrityError> {
    let table =
        toml::from_str::<toml::Table>(source).map_err(|_| ExternalIntegrityError::FieldSyntax)?;
    if table.get("kind").and_then(toml::Value::as_str) != Some("upstream-release-metadata") {
        return Err(ExternalIntegrityError::FieldSyntax);
    }
    let mut output = Vec::new();
    for name in [
        "tag",
        "asset",
        "checksum_algorithm",
        "checksum",
        "signature",
    ] {
        let value = table
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or(ExternalIntegrityError::FieldSyntax)?;
        output.push(field(ExternalEcosystem::Upstream, name, value));
    }
    Ok(output)
}

/// Parses only exact allowed external metadata fields and retains values opaquely.
///
/// No returned type implements conversion to `SemanticDigest`; the shell must
/// leave interpretation and verification to the owning ecosystem.
///
/// # Errors
///
/// Rejects broad exceptions, semantic promotion, unknown ecosystems, and
/// malformed or non-field-specific syntax.
pub fn parse_opaque_external_integrity(
    ecosystem: &str,
    source: &str,
) -> Result<Vec<OpaqueExternalIntegrityField>, ExternalIntegrityError> {
    if source.contains("[[exceptions]]")
        || source.contains("allow_all_hash_strings")
        || source.contains("always-external")
    {
        return Err(ExternalIntegrityError::BroadException);
    }
    if source.contains("SemanticDigest") {
        return Err(ExternalIntegrityError::SemanticPromotion);
    }
    let mut output = match ecosystem {
        "cargo" => parse_cargo(source)?,
        "nix" => parse_nix(source)?,
        "git" => parse_git(source)?,
        "upstream" => parse_upstream(source)?,
        "exception" => return Err(ExternalIntegrityError::BroadException),
        _ => return Err(ExternalIntegrityError::FieldSyntax),
    };
    output.sort_unstable();
    Ok(output)
}
