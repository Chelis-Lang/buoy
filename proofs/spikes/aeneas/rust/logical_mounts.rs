#![forbid(unsafe_code)]

pub struct LogicalProjection {
    pub prefix_head: u8,
    pub prefix_tail: u8,
    pub relative: u8,
}

pub fn logical_projection(prefix_head: u8, prefix_tail: u8, relative: u8) -> LogicalProjection {
    LogicalProjection {
        prefix_head,
        prefix_tail,
        relative,
    }
}

pub struct AttachmentKeyInput {
    pub field_tag: u8,
    pub relation: u64,
}

pub fn attachment_key_input(relation: u64) -> AttachmentKeyInput {
    AttachmentKeyInput {
        field_tag: 1,
        relation,
    }
}
