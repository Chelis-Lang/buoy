// SPG-ADAPTER-001 is only a comment.
const TEXT: &str = "SPG-ADAPTER-001 in a string";

macro_rules! mention {
    () => { "SPG-ADAPTER-001 in a macro" };
}

#[cfg(any())]
#[spec_provenance::carrier(syntax = "v1", id = "CAR-DISABLED")]
#[test]
fn disabled_carrier() {}
