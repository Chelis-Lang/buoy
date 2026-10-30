#![doc = "Compatibility CLI delegation and deprecation-warning control."]
#![allow(clippy::expect_used)]

use std::process::Command;

#[test]
fn compatibility_binary_warns_and_delegates_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_spec-provenance"))
        .output()
        .expect("compatibility binary executes");
    assert_eq!(output.status.code(), Some(64));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("`spec-provenance` is deprecated; use `buoy`")
    );
}
