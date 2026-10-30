#![no_main]

use buoy::core::{
    AggregationInput, SchemaImpactDependency, SchemaImpactIndex, SchemaImpactKey, SchemaImpactNode,
    SemanticDigest, Verdict, aggregate_assurance, schema_change_impacts,
};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 4_096;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let byte = data.first().copied().unwrap_or_default();
    let verdicts = data
        .iter()
        .take(16)
        .map(|value| match value % 5 {
            0 => Verdict::NotRun,
            1 => Verdict::Pass,
            2 => Verdict::Fail,
            3 => Verdict::Error,
            _ => Verdict::Timeout,
        })
        .collect();
    let _ = aggregate_assurance(&AggregationInput {
        linked: byte & 1 != 0,
        required_fresh: byte & 2 != 0,
        required_selected: byte & 4 != 0,
        verdicts,
        exhaustive: byte & 8 != 0,
        formal: byte & 16 != 0,
        structural: byte & 32 != 0,
        reviewed: byte & 64 != 0,
        debt: byte & 128 != 0,
        waiver: data.get(1).is_some_and(|value| value & 1 != 0),
        reaffirmation: data.get(1).is_some_and(|value| value & 2 != 0),
    });

    let target = SchemaImpactKey::new("atom-object", "FUZZ-ATOM");
    let source = SchemaImpactKey::new("evidence-carrier", "FUZZ-CARRIER");
    let (Ok(target), Ok(source)) = (target, source) else {
        return;
    };
    let dependency = SchemaImpactDependency::new("evidence-carrier-atoms", target.clone());
    let Ok(dependency) = dependency else {
        return;
    };
    let base_target = SchemaImpactNode::new(
        target.clone(),
        SemanticDigest::xxh3_128([byte; 16]),
        Vec::new(),
    );
    let head_target = SchemaImpactNode::new(
        target,
        SemanticDigest::xxh3_128([byte.wrapping_add(1); 16]),
        Vec::new(),
    );
    let source_node = SchemaImpactNode::new(
        source,
        SemanticDigest::xxh3_128([data.get(1).copied().unwrap_or_default(); 16]),
        vec![dependency],
    );
    let (Ok(base_target), Ok(head_target), Ok(source_node)) =
        (base_target, head_target, source_node)
    else {
        return;
    };
    let base = SchemaImpactIndex::new(vec![base_target, source_node.clone()]);
    let head = SchemaImpactIndex::new(vec![head_target, source_node]);
    if let (Ok(base), Ok(head)) = (base, head) {
        let impacts = schema_change_impacts(&base, &head);
        assert_eq!(impacts.len(), 2);
        assert_eq!(impacts.iter().filter(|impact| impact.direct()).count(), 1);
    }
});
