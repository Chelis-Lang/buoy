//! Generate/check dependency-independent malformed canonical byte vectors.

#![allow(clippy::expect_used, clippy::too_many_lines, clippy::unnecessary_wraps)]

use std::env;
use std::fmt::Write as _;
use std::fs;

const GOLDEN: &str = "fixtures/hardening/v2/malformed-canonical-vectors.tsv";

type CheckResult<T> = Result<T, String>;

struct Case {
    id: &'static str,
    bytes: Vec<u8>,
    max_bytes: u64,
    max_nesting: u16,
    max_values: u64,
    code: &'static str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("malformed-vectors: {error}");
        std::process::exit(1);
    }
}

fn run() -> CheckResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments == ["--coverage-control"] {
        return coverage_control();
    }
    run_with(&arguments, GOLDEN)
}

fn run_with(arguments: &[String], golden: &str) -> CheckResult<()> {
    let write = match arguments {
        [argument] if argument == "--write" => true,
        [argument] if argument == "--check" => false,
        _ => {
            return Err(String::from(
                "usage: generate_malformed_vectors <--write|--check>",
            ));
        }
    };
    let cases = cases();
    let rendered = render(&cases);
    if write {
        fs::write(golden, rendered).map_err(|error| format!("{golden}: {error}"))?;
    } else {
        let current = fs::read_to_string(golden).map_err(|error| format!("{golden}: {error}"))?;
        if current != rendered {
            return Err(format!("{golden} is stale; regenerate and review it"));
        }
    }
    println!("malformed-vectors: ok cases={}", cases.len());
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn coverage_control() -> CheckResult<()> {
    let scratch = env::temp_dir().join(format!("buoy-malformed-control-{}", std::process::id()));
    fs::create_dir_all(&scratch).expect("malformed control directory must be writable");
    let written = scratch.join("written.tsv");
    run_with(&[String::from("--write")], &written.to_string_lossy())
        .expect("malformed control write must succeed");
    run_with(&[], GOLDEN).expect_err("usage control must reject");
    run_with(&[String::from("--write")], &scratch.to_string_lossy())
        .expect_err("directory write must reject");
    run_with(
        &[String::from("--check")],
        &scratch.join("missing").to_string_lossy(),
    )
    .expect_err("missing golden must reject");
    let stale = scratch.join("stale.tsv");
    fs::write(&stale, "stale").expect("stale malformed control must write");
    run_with(&[String::from("--check")], &stale.to_string_lossy())
        .expect_err("stale golden must reject");
    fs::remove_dir_all(&scratch).expect("malformed control cleanup must succeed");
    Ok(())
}

fn cases() -> Vec<Case> {
    let u64_one = tlv(2, &1_u64.to_be_bytes());
    let u64_two = tlv(2, &2_u64.to_be_bytes());
    let mut duplicate_record = 2_u64.to_be_bytes().to_vec();
    duplicate_record.extend_from_slice(&1_u16.to_be_bytes());
    duplicate_record.extend_from_slice(&u64_one);
    duplicate_record.extend_from_slice(&1_u16.to_be_bytes());
    duplicate_record.extend_from_slice(&u64_two);
    let mut unordered_record = 2_u64.to_be_bytes().to_vec();
    unordered_record.extend_from_slice(&2_u16.to_be_bytes());
    unordered_record.extend_from_slice(&u64_one);
    unordered_record.extend_from_slice(&1_u16.to_be_bytes());
    unordered_record.extend_from_slice(&u64_two);
    let mut omitted_record = 2_u64.to_be_bytes().to_vec();
    omitted_record.extend_from_slice(&1_u16.to_be_bytes());
    omitted_record.extend_from_slice(&u64_one);
    let mut zero_record = 1_u64.to_be_bytes().to_vec();
    zero_record.extend_from_slice(&0_u16.to_be_bytes());
    zero_record.extend_from_slice(&u64_one);
    let mut duplicate_set = 2_u64.to_be_bytes().to_vec();
    duplicate_set.extend_from_slice(&u64_one);
    duplicate_set.extend_from_slice(&u64_one);
    let mut unordered_set = 2_u64.to_be_bytes().to_vec();
    unordered_set.extend_from_slice(&u64_two);
    unordered_set.extend_from_slice(&u64_one);
    let mut invalid_enum = 0_u16.to_be_bytes().to_vec();
    invalid_enum.extend(tlv(9, &[]));
    let mut enum_nonoption = 1_u16.to_be_bytes().to_vec();
    enum_nonoption.extend(tlv(1, &[]));
    let mut digest_algorithm = 1_u16.to_be_bytes().to_vec();
    digest_algorithm.extend_from_slice(&16_u16.to_be_bytes());
    digest_algorithm.extend_from_slice(&[0; 16]);
    let mut digest_length = 2_u16.to_be_bytes().to_vec();
    digest_length.extend_from_slice(&15_u16.to_be_bytes());
    digest_length.extend_from_slice(&[0; 16]);
    let nested = tlv(10, &tlv(10, &tlv(0, &[])));
    let mut two_values = 2_u64.to_be_bytes().to_vec();
    two_values.extend_from_slice(&tlv(0, &[]));
    two_values.extend_from_slice(&tlv(1, &[]));
    let mut trailing = tlv(0, &[]);
    trailing.push(0);
    vec![
        case("unknown-type-tag", tlv(13, &[]), "DET-ENC-UNKNOWN-TYPE-TAG"),
        case("truncated-header", vec![0, 0], "DET-ENC-LENGTH"),
        case(
            "declared-length-overflow",
            [vec![4], u64::MAX.to_be_bytes().to_vec()].concat(),
            "DET-ENC-LENGTH",
        ),
        case("fixed-length-mismatch", tlv(0, &[0]), "DET-ENC-LENGTH"),
        case(
            "record-duplicate-field",
            tlv(6, &duplicate_record),
            "DET-ENC-RECORD-TAGS",
        ),
        case(
            "record-out-of-order",
            tlv(6, &unordered_record),
            "DET-ENC-RECORD-TAGS",
        ),
        case(
            "record-omitted-field",
            tlv(6, &omitted_record),
            "DET-ENC-RECORD-TAGS",
        ),
        case(
            "record-zero-field-tag",
            tlv(6, &zero_record),
            "DET-ENC-RECORD-TAGS",
        ),
        case("set-duplicate", tlv(8, &duplicate_set), "DET-ORDER-SET"),
        case("set-out-of-order", tlv(8, &unordered_set), "DET-ORDER-SET"),
        case(
            "text-non-nfc",
            tlv(5, &[b'e', 0xcc, 0x81]),
            "DET-ENC-TEXT-NFC",
        ),
        case("text-invalid-utf8", tlv(5, &[0xff]), "DET-ENC-TEXT-NFC"),
        case("option-none-payload", tlv(9, &[0]), "DET-ENC-OPTION"),
        case("option-some-empty", tlv(10, &[]), "DET-ENC-OPTION"),
        case("enum-zero-tag", tlv(11, &invalid_enum), "DET-ENC-ENUM"),
        case("enum-nonoption", tlv(11, &enum_nonoption), "DET-ENC-ENUM"),
        case(
            "digest-unknown-algorithm",
            tlv(12, &digest_algorithm),
            "DET-ENC-DIGEST",
        ),
        case(
            "digest-declared-length",
            tlv(12, &digest_length),
            "DET-ENC-DIGEST",
        ),
        case("digest-payload-length", tlv(12, &[0; 35]), "DET-ENC-DIGEST"),
        case("trailing-bytes", trailing, "DET-ENC-TRAILING-BYTES"),
        Case {
            id: "byte-budget",
            max_bytes: 8,
            ..case("unused", tlv(0, &[]), "DET-ENC-BUDGET-BYTES")
        },
        Case {
            id: "nesting-budget",
            bytes: nested,
            max_nesting: 2,
            ..case("unused", Vec::new(), "DET-ENC-BUDGET-NESTING")
        },
        Case {
            id: "value-budget",
            bytes: tlv(7, &two_values),
            max_values: 2,
            ..case("unused", Vec::new(), "DET-ENC-BUDGET-VALUES")
        },
        Case {
            id: "collection-count-budget",
            bytes: tlv(7, &u64::MAX.to_be_bytes()),
            max_values: 100,
            ..case("unused", Vec::new(), "DET-ENC-BUDGET-VALUES")
        },
    ]
}

fn case(id: &'static str, bytes: Vec<u8>, code: &'static str) -> Case {
    Case {
        id,
        bytes,
        max_bytes: 1_048_576,
        max_nesting: 128,
        max_values: 1_000,
        code,
    }
}

fn tlv(tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut output = vec![tag];
    output.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    output.extend_from_slice(payload);
    output
}

fn render(cases: &[Case]) -> String {
    let mut output =
        String::from("schema\tid\thex\tmax_bytes\tmax_nesting\tmax_values\texpected_code\n");
    for case in cases {
        writeln!(
            &mut output,
            "spec-provenance-malformed-vector/v2\t{}\t{}\t{}\t{}\t{}\t{}",
            case.id,
            hex(&case.bytes),
            case.max_bytes,
            case.max_nesting,
            case.max_values,
            case.code,
        )
        .expect("writing a malformed vector into a String is infallible");
    }
    output
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
