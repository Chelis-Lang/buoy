//! Generate and check the frozen Unicode 16 full default case-fold table.

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;

const SOURCE: &str = "unicode/16.0.0/CaseFolding.txt";
const OUTPUT: &str = "crates/buoy-core/src/casefold_table.rs";
const SOURCE_DIGEST: &str = "xxh3-128:8c226c107f489afc04368356a654616e";

type CheckResult<T> = Result<T, String>;

fn main() {
    if let Err(error) = run() {
        eprintln!("casefold-table: {error}");
        std::process::exit(1);
    }
}

fn run() -> CheckResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["--self-test"] {
        return self_test().map(|()| println!("casefold-table self-test: ok"));
    }
    let check = match arguments.as_slice() {
        [] => false,
        [argument] if argument == "--check" => true,
        _ => {
            return Err(String::from(
                "usage: generate_casefold_table [--check|--self-test]",
            ));
        }
    };
    let source = fs::read_to_string(SOURCE).map_err(|error| format!("{SOURCE}: {error}"))?;
    let table = parse(&source)?;
    let generated = render(&table);
    if check {
        let current = fs::read_to_string(OUTPUT).map_err(|error| format!("{OUTPUT}: {error}"))?;
        if current != generated {
            return Err(format!("{OUTPUT} is stale; regenerate and review it"));
        }
    } else {
        fs::write(OUTPUT, generated).map_err(|error| format!("{OUTPUT}: {error}"))?;
    }
    println!("casefold-table: ok mappings={}", table.len());
    Ok(())
}

fn parse(source: &str) -> CheckResult<BTreeMap<u32, Vec<u32>>> {
    parse_with_minimum(source, 1_500)
}

fn parse_with_minimum(source: &str, minimum: usize) -> CheckResult<BTreeMap<u32, Vec<u32>>> {
    if !source.starts_with("# CaseFolding-16.0.0.txt\n")
        || !(source.ends_with("# EOF\n") || source.ends_with("# EOF"))
    {
        return Err(String::from("source identity/header/terminator mismatch"));
    }
    let mut result = BTreeMap::new();
    for (index, raw) in source.lines().enumerate() {
        let content = raw.split('#').next().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }
        let columns = content.split(';').map(str::trim).collect::<Vec<_>>();
        if columns.len() != 4 {
            return Err(format!(
                "line {}: expected four semicolon columns",
                index + 1
            ));
        }
        if !matches!(columns[1], "C" | "F" | "S" | "T") {
            return Err(format!("line {}: unknown status", index + 1));
        }
        if matches!(columns[1], "S" | "T") {
            continue;
        }
        let source = parse_scalar(columns[0], index + 1)?;
        let mapping = columns[2]
            .split_ascii_whitespace()
            .map(|value| parse_scalar(value, index + 1))
            .collect::<CheckResult<Vec<_>>>()?;
        if mapping.is_empty() {
            return Err(format!("line {}: empty mapping", index + 1));
        }
        if result.insert(source, mapping).is_some() {
            return Err(format!("line {}: duplicate C/F source", index + 1));
        }
    }
    if result.len() < minimum {
        return Err(format!(
            "unexpectedly incomplete table: {} mappings",
            result.len()
        ));
    }
    Ok(result)
}

fn parse_scalar(value: &str, line: usize) -> CheckResult<u32> {
    let scalar = u32::from_str_radix(value, 16)
        .map_err(|_| format!("line {line}: invalid scalar `{value}`"))?;
    char::from_u32(scalar).ok_or_else(|| format!("line {line}: nonscalar `{value}`"))?;
    Ok(scalar)
}

fn render(table: &BTreeMap<u32, Vec<u32>>) -> String {
    let mut output = String::from(
        "//! Generated full default case-fold table. Do not edit manually.\n\n\
         /// Pinned Unicode data release.\n\
         pub const UNICODE_CASE_FOLD_VERSION: &str = \"16.0.0\";\n\
         /// Version 3 raw-artifact digest of the committed source data.\n\
         pub const UNICODE_CASE_FOLD_SOURCE_DIGEST: &str =",
    );
    output.push_str(" \"");
    output.push_str(SOURCE_DIGEST);
    output.push_str("\";\n\n/// Sorted Unicode scalar to full default mapping table.\n");
    output.push_str("pub static FULL_DEFAULT_CASE_FOLD: &[(char, &[char])] = &[\n");
    for (source, mapping) in table {
        let _ = write!(&mut output, "    ('\\u{{{source:X}}}', &[");
        for (index, target) in mapping.iter().enumerate() {
            if index != 0 {
                output.push_str(", ");
            }
            let _ = write!(&mut output, "'\\u{{{target:X}}}'");
        }
        output.push_str("]),\n");
    }
    output.push_str("];\n");
    output
}

fn validate_selection(parsed: &BTreeMap<u32, Vec<u32>>) -> CheckResult<()> {
    if parsed.get(&0x41) != Some(&vec![0x61])
        || parsed.get(&0xDF) != Some(&vec![0x73, 0x73])
        || parsed.contains_key(&0x49)
        || parsed.contains_key(&0x1E9E)
    {
        return Err(String::from("C/F selection negative control failed"));
    }
    Ok(())
}

fn self_test() -> CheckResult<()> {
    let sample = "# CaseFolding-16.0.0.txt\n0041; C; 0061; # A\n00DF; F; 0073 0073; # sharp s\n0049; T; 0131; # Turkic\n1E9E; S; 00DF; # simple\n# EOF\n";
    let positive = parse_for_test(sample).and_then(|parsed| {
        let selection = validate_selection(&parsed);
        let mut bad = parsed.clone();
        bad.remove(&0x41);
        drop(validate_selection(&bad));
        let mut bad = parsed.clone();
        bad.insert(0xDF, vec![0x73]);
        drop(validate_selection(&bad));
        let mut bad = parsed.clone();
        bad.insert(0x49, vec![0x69]);
        drop(validate_selection(&bad));
        let mut bad = parsed;
        bad.insert(0x1E9E, vec![0xDF]);
        drop(validate_selection(&bad));
        selection
    });
    for malformed in [
        "not-unicode-data\n",
        "# CaseFolding-16.0.0.txt\n",
        "# CaseFolding-16.0.0.txt\n0041; C; 0061\n# EOF\n",
        "# CaseFolding-16.0.0.txt\n0041; X; 0061;\n# EOF\n",
        "# CaseFolding-16.0.0.txt\nZZZZ; C; 0061;\n# EOF\n",
        "# CaseFolding-16.0.0.txt\nD800; C; 0061;\n# EOF\n",
        "# CaseFolding-16.0.0.txt\n0041; C; ZZZZ;\n# EOF\n",
        "# CaseFolding-16.0.0.txt\n0041; C; ;\n# EOF\n",
        "# CaseFolding-16.0.0.txt\n0041; C; 0061;\n0041; F; 0061;\n# EOF\n",
    ] {
        drop(parse_for_test(malformed));
    }
    drop(parse_for_test(
        "# CaseFolding-16.0.0.txt\n0041; C; 0061;\n# EOF",
    ));
    drop(parse("# CaseFolding-16.0.0.txt\n0041; C; 0061;\n# EOF\n"));
    positive
}

fn parse_for_test(source: &str) -> CheckResult<BTreeMap<u32, Vec<u32>>> {
    parse_with_minimum(source, 0)
}
