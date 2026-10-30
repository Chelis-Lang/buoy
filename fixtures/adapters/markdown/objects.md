```spec-provenance-surface-v1
id = "SURFACE-ADAPTER-001"
member_key_schema = { schema = "adapter.member-key/v1", bytes_hex = "736368656d61" }
disposition_vocabulary = { schema = "adapter.disposition/v1", bytes_hex = "766f636162756c617279" }
member_keys = [{ schema = "adapter.member/v1", bytes_hex = "61" }]
generation = 1
policies = []
dispositions = [{ key = { schema = "adapter.member/v1", bytes_hex = "61" }, disposition = "normative", payload = { schema = "adapter.disposition/v1", bytes_hex = "6e6f726d6174697665" }, authority = { id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" } }]
```

```spec-provenance-policy-v1
id = "POL-ADAPTER-001"
scope = { schema = "adapter.policy-scope/v1", bytes_hex = "616c6c" }
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "adapter-ratchet"
authority_selectors = []
active_approval = { authority = "maintainers", record = "reviews/policy.md", digest = "xxh3-128:12121212121212121212121212121212" }
lifecycle = "active"
replacements = []
```

```spec-provenance-debt-v1
id = "DEBT-ADAPTER-001"
obligations = [{ policy_id = "POL-ADAPTER-001", kind = "freshness", subject = { kind = "atom", id = "ATOM-ADAPTER-001", digest = "xxh3-128:13131313131313131313131313131313" } }]
objects = [{ kind = "atom", id = "ATOM-ADAPTER-001", digest = "xxh3-128:13131313131313131313131313131313" }]
rationale = "The adapter fixture records visible debt."
owner = "adapter-owner"
remediation = "Refresh the exact bound artifact."
discharge_condition = { schema = "adapter.discharge/v1", bytes_hex = "6672657368" }
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```

```spec-provenance-waiver-v1
id = "WAIVER-ADAPTER-001"
obligations = [{ policy_id = "POL-ADAPTER-001", kind = "execution", subject = { kind = "atom", id = "ATOM-ADAPTER-001", digest = "xxh3-128:13131313131313131313131313131313" } }]
objects = [{ kind = "atom", id = "ATOM-ADAPTER-001", digest = "xxh3-128:13131313131313131313131313131313" }]
rationale = "The fixture authorizes process continuation only."
owner = "adapter-owner"
discharge_condition = { schema = "adapter.discharge/v1", bytes_hex = "70617373" }
active_approval = { authority = "maintainers", record = "reviews/waiver.md", digest = "xxh3-128:14141414141414141414141414141414" }
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```

```spec-provenance-reaffirmation-v1
id = "REAFFIRM-ADAPTER-001"
affected_object_kind = "atom"
affected_object_id = "ATOM-ADAPTER-001"
base_object_digest = "xxh3-128:15151515151515151515151515151515"
head_object_digest = "xxh3-128:16161616161616161616161616161616"
review_approval = { authority = "maintainers", record = "reviews/reaffirmation.md", digest = "xxh3-128:17171717171717171717171717171717" }
review_statement = "The exact changed atom remains intended."
```

```spec-provenance-derivation-v1
id = "DERIV-ADAPTER-001"
sources = [{ kind = "repository-artifact", path = "src/input.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "xxh3-128:18181818181818181818181818181818" }]
targets = [{ kind = "repository-artifact", path = "generated/output.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "xxh3-128:19191919191919191919191919191919" }]
tool = { id = "adapter-generator", version = "1.0.0", executable = "xxh3-128:20202020202020202020202020202020", configuration = "xxh3-128:21212121212121212121212121212121" }
configuration = "xxh3-128:22222222222222222222222222222222"
oracle = { kind = "oracle", id = "ORACLE-ADAPTER-001", digest = "xxh3-128:23232323232323232323232323232323" }
relation = "generated-from"
relation_scope = { schema = "adapter.relation-scope/v1", bytes_hex = "6578616374" }
exclusions = ["Compiler backend behavior is excluded."]
```

```spec-provenance-assumption-v1
id = "ASSUMPTION-ADAPTER-001"
statement = "The fixture input is immutable during pure materialization."
scope = { schema = "adapter.assumption-scope/v1", bytes_hex = "61646170746572" }
exclusions = ["Operating-system behavior is excluded."]
approval = { authority = "maintainers", record = "reviews/assumption.md", digest = "xxh3-128:24242424242424242424242424242424" }
```

```spec-provenance-finite-model-v1
id = "MODEL-ADAPTER-001"
state_space = { schema = "adapter.model-state/v1", bytes_hex = "73302c7331" }
event_alphabet = { schema = "adapter.model-event/v1", bytes_hex = "73746570" }
transition_relation = { schema = "adapter.model-transition/v1", bytes_hex = "73302d3e7331" }
initial_states = [{ schema = "adapter.model-state/v1", bytes_hex = "7330" }]
bound = { schema = "adapter.model-bound/v1", bytes_hex = "32" }
checked_invariants = [{ id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" }]
assumptions = [{ kind = "assumption", id = "ASSUMPTION-ADAPTER-001", digest = "xxh3-128:25252525252525252525252525252525" }]
exclusions = ["Unbounded states are excluded."]
```

```spec-provenance-formal-property-v1
id = "PROPERTY-ADAPTER-001"
atoms = [{ id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" }]
property = { schema = "adapter.formal-property/v1", bytes_hex = "696e76617269616e74" }
implementation_abstraction = { schema = "adapter.implementation-abstraction/v1", bytes_hex = "6d6f64656c" }
assumptions = [{ kind = "assumption", id = "ASSUMPTION-ADAPTER-001", digest = "xxh3-128:25252525252525252525252525252525" }]
exclusions = ["Parser correctness is excluded."]
correspondence_approval = { authority = "maintainers", record = "reviews/property.md", digest = "xxh3-128:26262626262626262626262626262626" }
```

```spec-provenance-proof-record-v1
id = "PROOF-ADAPTER-001"
atoms = [{ id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" }]
property = { kind = "formal-property", id = "PROPERTY-ADAPTER-001", digest = "xxh3-128:27272727272727272727272727272727" }
implementation = { kind = "repository-artifact", path = "src/kernel.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "xxh3-128:28282828282828282828282828282828" }
assumptions = [{ kind = "assumption", id = "ASSUMPTION-ADAPTER-001", digest = "xxh3-128:25252525252525252525252525252525" }]
translators = []
verifier = { id = "adapter-verifier", version = "1.0.0", executable = "xxh3-128:29292929292929292929292929292929", configuration = "xxh3-128:30303030303030303030303030303030" }
verifier_configuration = "xxh3-128:31313131313131313131313131313131"
external_models = []
generated = []
proof_sources = [{ kind = "repository-artifact", path = "proofs/adapter.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "xxh3-128:32323232323232323232323232323232" }]
verdict = "pass"
scope = { schema = "adapter.proof-scope/v1", bytes_hex = "6b65726e656c" }
exclusions = ["Compiler and operating-system behavior are excluded."]
```
