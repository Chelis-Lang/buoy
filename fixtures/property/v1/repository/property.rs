fn hegel_control_valid_property(input: &[u8]) { assert_eq!(production::semantic(input), reference::semantic(input)); }
fn hegel_control_other_property(input: &[u8]) { assert_eq!(production::other(input), reference::other(input)); }
