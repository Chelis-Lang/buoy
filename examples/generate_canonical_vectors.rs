//! Generate/check independent canonical vectors expanded from the frozen schema inventory.

#![allow(
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::unnecessary_map_on_constructor,
    clippy::unnecessary_wraps
)]

use buoy_core::{ActiveAlgorithm, ActiveAlgorithmError, DigestAlgorithm};
use buoy_reference::{Value, semantic_preimage};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;

const INVENTORY: &str = "schema/v3/inventory.tsv";
const GOLDEN: &str = "fixtures/hardening/v3/canonical-vectors.tsv";

type CheckResult<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct Row {
    category: String,
    parent: String,
    name: String,
    tag: Option<u16>,
    shape: String,
    cardinality: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("canonical-vectors: {error}");
        std::process::exit(1);
    }
}

fn run() -> CheckResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments == ["--coverage-control"] {
        return coverage_control();
    }
    run_with(&arguments, INVENTORY, GOLDEN)
}

fn run_with(arguments: &[String], inventory: &str, golden: &str) -> CheckResult<()> {
    let write = match arguments {
        [argument] if argument == "--write" => true,
        [argument] if argument == "--check" => false,
        _ => {
            return Err(String::from(
                "usage: generate_canonical_vectors <--write|--check>",
            ));
        }
    };
    let source = fs::read_to_string(inventory).map_err(|error| format!("{inventory}: {error}"))?;
    let rows = parse_inventory(&source)?;
    let rendered = render(&rows)?;
    if write {
        fs::write(golden, rendered).map_err(|error| format!("{golden}: {error}"))?;
    } else {
        let current = fs::read_to_string(golden).map_err(|error| format!("{golden}: {error}"))?;
        if current != rendered {
            return Err(format!("{golden} is stale; regenerate and review it"));
        }
    }
    println!("canonical-vectors: ok inventory-rows={}", rows.len());
    Ok(())
}

fn coverage_control() -> CheckResult<()> {
    let scratch = env::temp_dir().join(format!("buoy-canonical-control-{}", std::process::id()));
    fs::create_dir_all(&scratch).expect("canonical control directory must be writable");
    let written = scratch.join("written.tsv");
    run_with(
        &[String::from("--write")],
        INVENTORY,
        &written.to_string_lossy(),
    )
    .expect("canonical control write must succeed");
    run_with(&[], INVENTORY, GOLDEN).expect_err("usage control must reject");
    run_with(
        &[String::from("--check")],
        &scratch.join("missing-inventory").to_string_lossy(),
        GOLDEN,
    )
    .expect_err("missing inventory must reject");
    run_with(
        &[String::from("--check")],
        INVENTORY,
        &scratch.join("missing-golden").to_string_lossy(),
    )
    .expect_err("missing golden must reject");
    let stale = scratch.join("stale.tsv");
    fs::write(&stale, "stale").expect("stale canonical control must write");
    run_with(
        &[String::from("--check")],
        INVENTORY,
        &stale.to_string_lossy(),
    )
    .expect_err("stale golden must reject");

    let malformed_inventory = scratch.join("malformed-inventory.tsv");
    fs::write(&malformed_inventory, "wrong-header\n")
        .expect("malformed inventory control must write");
    run_with(
        &[String::from("--check")],
        &malformed_inventory.to_string_lossy(),
        GOLDEN,
    )
    .expect_err("inventory parse propagation must reject");
    let render_inventory = scratch.join("render-error-inventory.tsv");
    fs::write(
        &render_inventory,
        "category\tparent\tname\ttag\tshape\tcardinality\tcollection\trule\nfield\tDuplicate\ta\t1\ttext\trequired\tordered\trule\nfield\tDuplicate\tb\t1\ttext\trequired\tordered\trule\nnested\t-\tDuplicate\t-\trecord\trequired\tordered\trule\n",
    )
    .expect("render error inventory control must write");
    run_with(
        &[String::from("--check")],
        &render_inventory.to_string_lossy(),
        GOLDEN,
    )
    .expect_err("render propagation must reject");
    run_with(
        &[String::from("--write")],
        INVENTORY,
        &scratch.to_string_lossy(),
    )
    .expect_err("directory output must reject");

    for malformed in [
        "",
        "wrong-header\n",
        "category\tparent\tname\ttag\tshape\tcardinality\tcollection\trule\nshort\n",
        "category\tparent\tname\ttag\tshape\tcardinality\tcollection\trule\nfield\tP\tx\tbad\ttext\trequired\tordered\trule\n",
    ] {
        parse_inventory(malformed).expect_err("malformed inventory must reject");
    }

    let row = |category: &str,
               parent: &str,
               name: &str,
               tag: Option<u16>,
               shape: &str,
               cardinality: &str| Row {
        category: category.to_owned(),
        parent: parent.to_owned(),
        name: name.to_owned(),
        tag,
        shape: shape.to_owned(),
        cardinality: cardinality.to_owned(),
    };
    let schema_rows = vec![
        row("field", "Known", "value", Some(1), "text", "required"),
        row("enum-variant", "Choice", "a", Some(2), "unit", "required"),
        row("enum-variant", "Choice", "b", Some(1), "unit", "required"),
        row(
            "enum-variant",
            "Choice",
            "untagged",
            None,
            "unit",
            "required",
        ),
        row("nested", "-", "Inline", None, "text", "required"),
    ];
    let schema = Schema::new(&schema_rows);
    for (shape, cardinality) in [
        ("set:text", "nonempty"),
        ("set:text", "optional"),
        ("option:text", "optional"),
        ("nested:Known", "required"),
        ("nested:Inline", "required"),
        ("domain:Known", "required"),
        ("report-fact:Known", "required"),
        ("enum:Choice", "required"),
        ("digest", "required"),
        ("digest:semantic", "required"),
        ("Known", "required"),
        ("text", "required"),
        ("nfc-utf8", "required"),
        ("bytes", "required"),
        ("u64", "required"),
        ("u64-be", "required"),
        ("i64", "required"),
        ("i64-twos-complement-be", "required"),
        ("unit", "required"),
        ("empty", "required"),
        ("set-control", "required"),
        ("sequence-control", "required"),
        ("record-control", "required"),
    ] {
        schema
            .shape(shape, cardinality, &mut BTreeSet::new())
            .expect("reviewed shape control must succeed");
    }
    for shape in ["set:unsupported", "enum:Missing", "unsupported"] {
        schema
            .shape(shape, "nonempty", &mut BTreeSet::new())
            .expect_err("invalid shape must reject");
    }
    schema
        .record("Missing", &mut BTreeSet::new())
        .expect_err("missing record must reject");
    let recursive_rows = vec![row(
        "field",
        "Recursive",
        "self",
        Some(1),
        "nested:Recursive",
        "required",
    )];
    Schema::new(&recursive_rows)
        .record("Recursive", &mut BTreeSet::new())
        .expect_err("recursive record must reject");
    let untagged_rows = vec![row("field", "Untagged", "value", None, "text", "required")];
    Schema::new(&untagged_rows)
        .record("Untagged", &mut BTreeSet::new())
        .expect_err("untagged field must reject");
    type_tag(99).expect_err("unknown type tag must reject");

    render(&[row(
        "type-tag",
        "-",
        "missing-tag",
        None,
        "unit",
        "required",
    )])
    .expect_err("missing type tag must reject");
    render(&[row(
        "type-tag",
        "-",
        "unknown-tag",
        Some(99),
        "unit",
        "required",
    )])
    .expect_err("unknown type tag must reject");
    render(&[row("nested", "-", "Missing", None, "record", "required")])
        .expect_err("missing nested record must reject");
    render(&[
        row("field", "Inline", "value", Some(1), "text", "required"),
        row("nested", "-", "Inline", None, "text", "required"),
    ])
    .expect("inline nested record must render");
    render(&[row("nested", "-", "Missing", None, "missing", "required")])
        .expect_err("missing inline nested shape must reject");
    render(&[row("domain", "-", "Missing", None, "record", "required")])
        .expect_err("missing domain record must reject");
    let duplicate_rows = vec![
        row("field", "Duplicate", "a", Some(1), "text", "required"),
        row("field", "Duplicate", "b", Some(1), "text", "required"),
        row("nested", "-", "Duplicate", None, "record", "required"),
    ];
    render(&duplicate_rows).expect_err("duplicate encoded fields must reject");
    let noncanonical = format!("Domain-e{}", '\u{301}');
    let domain_rows = vec![
        row("field", &noncanonical, "value", Some(1), "text", "required"),
        row("domain", "-", &noncanonical, None, "record", "required"),
    ];
    render(&domain_rows).expect_err("noncanonical domain must reject");
    let _ = Err::<(), _>(ActiveAlgorithmError::Unavailable(DigestAlgorithm::Xxh3_128))
        .map_err(active_algorithm_error)
        .expect_err("unavailable active-algorithm control must reject");
    let _ = Err::<(), _>(ActiveAlgorithmError::Inactive {
        active: DigestAlgorithm::Blake3_256,
        requested: DigestAlgorithm::Xxh3_128,
    })
    .map_err(digest_error)
    .expect_err("inactive digest control must reject");
    fs::remove_dir_all(&scratch).expect("canonical control cleanup must succeed");
    Ok(())
}

fn parse_inventory(source: &str) -> CheckResult<Vec<Row>> {
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| String::from("empty inventory"))?;
    if header != "category\tparent\tname\ttag\tshape\tcardinality\tcollection\trule" {
        return Err(String::from("inventory header mismatch"));
    }
    lines
        .enumerate()
        .map(|(index, line)| {
            let columns = line.split('\t').collect::<Vec<_>>();
            if columns.len() != 8 {
                return Err(format!("line {}: expected eight columns", index + 2));
            }
            let tag = if columns[3] == "-" {
                None
            } else {
                Some(
                    columns[3]
                        .parse::<u16>()
                        .map_err(|_| format!("line {}: invalid tag", index + 2))?,
                )
            };
            Ok(Row {
                category: columns[0].to_owned(),
                parent: columns[1].to_owned(),
                name: columns[2].to_owned(),
                tag,
                shape: columns[4].to_owned(),
                cardinality: columns[5].to_owned(),
            })
        })
        .collect()
}

struct Schema<'a> {
    fields: BTreeMap<&'a str, Vec<&'a Row>>,
    enums: BTreeMap<&'a str, u16>,
    nested_shapes: BTreeMap<&'a str, &'a str>,
}

impl<'a> Schema<'a> {
    fn new(rows: &'a [Row]) -> Self {
        let mut fields = BTreeMap::<&str, Vec<&Row>>::new();
        let mut enums = BTreeMap::new();
        let mut nested_shapes = BTreeMap::new();
        for row in rows {
            if row.category == "field" {
                fields.entry(&row.parent).or_default().push(row);
            } else if row.category == "nested" && row.shape != "record" {
                nested_shapes.insert(row.name.as_str(), row.shape.as_str());
            } else if row.category == "enum-variant" {
                if let Some(tag) = row.tag {
                    enums
                        .entry(row.parent.as_str())
                        .and_modify(|current: &mut u16| *current = (*current).min(tag))
                        .or_insert(tag);
                }
            }
        }
        for values in fields.values_mut() {
            values.sort_unstable_by_key(|row| row.tag);
        }
        Self {
            fields,
            enums,
            nested_shapes,
        }
    }

    fn record_fields(
        &self,
        name: &str,
        stack: &mut BTreeSet<String>,
    ) -> CheckResult<Vec<(u16, Value)>> {
        if !stack.insert(name.to_owned()) {
            return Err(format!("recursive sample shape at `{name}`"));
        }
        let rows = self
            .fields
            .get(name)
            .ok_or_else(|| format!("missing field table for `{name}`"))?;
        let mut fields = Vec::new();
        for row in rows {
            let tag = row
                .tag
                .ok_or_else(|| format!("untagged field `{name}/{}`", row.name))?;
            fields.push((tag, self.shape(&row.shape, &row.cardinality, stack)?));
        }
        stack.remove(name);
        Ok(fields)
    }

    fn record(&self, name: &str, stack: &mut BTreeSet<String>) -> CheckResult<Value> {
        Ok(Value::Record(self.record_fields(name, stack)?))
    }

    fn shape(
        &self,
        shape: &str,
        cardinality: &str,
        stack: &mut BTreeSet<String>,
    ) -> CheckResult<Value> {
        if let Some(inner) = shape.strip_prefix("set:") {
            let values = if cardinality == "nonempty" {
                vec![self.shape(inner, "required", stack)?]
            } else {
                Vec::new()
            };
            return Ok(Value::Set(values));
        }
        if shape.starts_with("option:") {
            return Ok(Value::None);
        }
        if let Some(name) = shape.strip_prefix("nested:") {
            return if let Some(nested_shape) = self.nested_shapes.get(name) {
                self.shape(nested_shape, cardinality, stack)
            } else {
                self.record(name, stack)
            };
        }
        if let Some(name) = shape
            .strip_prefix("domain:")
            .or_else(|| shape.strip_prefix("report-fact:"))
        {
            return self.record(name, stack);
        }
        if let Some(name) = shape.strip_prefix("enum:") {
            let tag = self
                .enums
                .get(name)
                .copied()
                .ok_or_else(|| format!("missing enum `{name}`"))?;
            return Ok(Value::Enum(tag, None));
        }
        if shape == "digest" || shape.starts_with("digest:") {
            return Ok(Value::Digest(2, vec![0x5a; 16]));
        }
        if self.fields.contains_key(shape) {
            return self.record(shape, stack);
        }
        match shape {
            "text" | "nfc-utf8" => Ok(Value::Text(String::from("v2"))),
            "bytes" => Ok(Value::Bytes(vec![0, 1, 255])),
            "u64" | "u64-be" => Ok(Value::Unsigned(1)),
            "i64" | "i64-twos-complement-be" => Ok(Value::Signed(-1)),
            "unit" | "empty" => Ok(Value::None),
            value if value.starts_with("set") => Ok(Value::Set(Vec::new())),
            value if value.starts_with("sequence") => Ok(Value::Sequence(Vec::new())),
            value if value.starts_with("record") => Ok(Value::Record(Vec::new())),
            other => Err(format!("unsupported sample shape `{other}`")),
        }
    }
}

fn render(rows: &[Row]) -> CheckResult<String> {
    let schema = Schema::new(rows);
    let mut output = String::from("schema\tkind\tid\tcanonical_hex\tsemantic_digest\n");
    for row in rows {
        let (vector, domain_fields) = match row.category.as_str() {
            "type-tag" => (
                Some(type_tag(
                    row.tag.ok_or_else(|| String::from("type tag absent"))?,
                )?),
                None,
            ),
            "domain" => {
                let fields = schema.record_fields(&row.name, &mut BTreeSet::new())?;
                (Some(Value::Record(fields.clone())), Some(fields))
            }
            "nested" | "report-fact" => {
                let vector =
                    if row.shape == "record" || schema.fields.contains_key(row.name.as_str()) {
                        schema.record(&row.name, &mut BTreeSet::new())?
                    } else {
                        schema.shape(&row.shape, &row.cardinality, &mut BTreeSet::new())?
                    };
                (Some(vector), None)
            }
            _ => (None, None),
        };
        let Some(vector) = vector else { continue };
        let bytes = vector
            .encode()
            .map_err(|error| format!("{}: {error:?}", row.name))?;
        let digest = if let Some(fields) = domain_fields {
            let preimage = semantic_preimage(2, &row.name, fields)
                .map_err(|error| format!("{} preimage: {error:?}", row.name))?;
            ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128)
                .expect("canonical vector generation requires the XXH3-128 adapter")
                .compute(DigestAlgorithm::Xxh3_128, &preimage)
                .expect("the parsed active member computes under its own adapter")
                .canonical_text()
        } else {
            String::from("-")
        };
        writeln!(
            &mut output,
            "spec-provenance-canonical-vector/v2\t{}\t{}\t{}\t{}",
            row.category,
            row.name,
            hex(&bytes),
            digest,
        )
        .expect("writing a canonical vector into a String is infallible");
    }
    Ok(output)
}

fn type_tag(tag: u16) -> CheckResult<Value> {
    match tag {
        0 => Ok(Value::False),
        1 => Ok(Value::True),
        2 => Ok(Value::Unsigned(u64::MAX)),
        3 => Ok(Value::Signed(i64::MIN)),
        4 => Ok(Value::Bytes(vec![0, 1, 255])),
        5 => Ok(Value::Text(String::from("é"))),
        6 => Ok(Value::Record(Vec::new())),
        7 => Ok(Value::Sequence(Vec::new())),
        8 => Ok(Value::Set(Vec::new())),
        9 => Ok(Value::None),
        10 => Ok(Value::Some(Box::new(Value::True))),
        11 => Ok(Value::Enum(1, None)),
        12 => Ok(Value::Digest(2, vec![0x5a; 16])),
        _ => Err(format!("unknown type tag {tag}")),
    }
}

fn active_algorithm_error(error: ActiveAlgorithmError) -> String {
    format!("XXH3-128 unavailable: {}", error.code())
}

fn digest_error(error: ActiveAlgorithmError) -> String {
    format!("canonical vector digest: {}", error.code())
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
