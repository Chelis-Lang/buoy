#![forbid(unsafe_code)]

pub fn source_item_kind_valid(path_components: u8, version: u8) -> bool {
    path_components >= 2 && version >= 1
}

pub struct ParsedItemFields {
    pub schema: u8,
    pub path: u8,
    pub source_item_kind: u8,
    pub semantic_bytes: u8,
}

pub fn parsed_item_fields(
    schema: u8,
    path: u8,
    source_item_kind: u8,
    semantic_bytes: u8,
) -> ParsedItemFields {
    ParsedItemFields {
        schema,
        path,
        source_item_kind,
        semantic_bytes,
    }
}
