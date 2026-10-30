import Integrity

open Aeneas
open Aeneas.Result
open Integrity

/-- The generated logical projection preserves the bounded prefix segments and relative segment. -/
theorem logical_projection_binds_prefix_and_relative
    (prefixHead prefixTail relative : Aeneas.Std.U8) :
    logical_mounts.logical_projection prefixHead prefixTail relative =
      ok {
        prefix_head := prefixHead,
        prefix_tail := prefixTail,
        relative := relative
      } := by
  rfl

/-- The generated attachment key input preserves the field tag and stable relation value. -/
theorem attachment_key_input_binds_relation (relation : Aeneas.Std.U64) :
    logical_mounts.attachment_key_input relation =
      ok { field_tag := 1#u8, relation := relation } := by
  rfl

/-- A terminal lifecycle state cannot transition to a live state. -/
theorem lifecycle_terminality (event : Event) :
    transition Lifecycle.Superseded event = ok none ∧
    transition Lifecycle.Withdrawn event = ok none := by
  cases event <;> constructor <;> rfl

/-- Omitting a declared surface member makes completeness false. -/
theorem surface_missing_member_is_incomplete
    (domain memberA authorityA authorityB : Bool) :
    surface_complete domain memberA false authorityA authorityB = ok false := by
  cases domain <;> cases memberA <;> cases authorityA <;> cases authorityB <;> rfl

/-- A stale atom or target binding prevents carrier and derivation freshness. -/
theorem stale_bindings_prevent_freshness
    (oracle configuration corpus item source tool derivationConfiguration
      derivationOracle consistencyPassed : Bool) :
    carrier_fresh false oracle configuration corpus item = ok false ∧
    derivation_fresh source false tool derivationConfiguration derivationOracle
      consistencyPassed = ok false := by
  cases oracle <;> cases configuration <;> cases corpus <;> cases item <;>
    cases source <;> cases tool <;> cases derivationConfiguration <;>
    cases derivationOracle <;> cases consistencyPassed <;> constructor <;> rfl

/-- Adding policy scope cannot remove a previously required member. -/
theorem policy_growth_is_monotonic
    (baseA baseB addedA addedB : Bool) :
    policy_monotonic baseA baseB (baseA || addedA) (baseB || addedB) = ok true := by
  cases baseA <;> cases baseB <;> cases addedA <;> cases addedB <;> rfl

/-- One missing transitive impact disposition prevents closure. -/
theorem missing_impact_prevents_closure (carrier surface proof : Bool) :
    impact_closed carrier surface false proof = ok false := by
  cases carrier <;> cases surface <;> cases proof <;> rfl

/-- Process-only records cannot alter the technical tested verdict. -/
theorem process_records_do_not_launder
    (technical current passed debt waiver reaffirmation : Bool) :
    aggregate_tested technical current passed debt waiver reaffirmation =
      aggregate_tested technical current passed false false false := by
  rfl

/-- The generated field constructor preserves both canonical header fields. -/
theorem field_header_binds_tag_and_length
    (tag : Aeneas.Std.U8) (length : Aeneas.Std.U64) :
    field_header tag length = ok { tag := tag, length := length } := by
  rfl

/-- The v3 envelope binds schema, encoding, and all registry algorithm tags together. -/
theorem versioned_envelope_binds_v3_registry :
    versioned_envelope_valid 3 3 2 = ok true ∧
    versioned_envelope_valid 3 3 3 = ok true ∧
    versioned_envelope_valid 3 3 4 = ok true ∧
    versioned_envelope_valid 2 3 2 = ok false ∧
    versioned_envelope_valid 3 2 2 = ok false ∧
    versioned_envelope_valid 3 3 1 = ok false := by
  constructor
  · rfl
  · constructor
    · rfl
    · constructor
      · rfl
      · constructor
        · rfl
        · constructor <;> rfl

/-- Canonical XXH128 bytes preserve big-endian byte order within high then low halves. -/
theorem xxh128_half_and_byte_order (low high : Octets) :
    canonical_xxh128_bytes low high = ok {
      b0 := high.b0, b1 := high.b1, b2 := high.b2, b3 := high.b3,
      b4 := high.b4, b5 := high.b5, b6 := high.b6, b7 := high.b7,
      b8 := low.b0, b9 := low.b1, b10 := low.b2, b11 := low.b3,
      b12 := low.b4, b13 := low.b5, b14 := low.b6, b15 := low.b7
    } := by
  rfl

/-- Exactly the frozen type tag, registry tags, and member lengths are accepted. -/
theorem parser_length_and_tag_invariants
    (typeTag algorithmTag length : Aeneas.Std.U8) :
    digest_shape_valid typeTag algorithmTag length = ok true ↔
      typeTag = 12 ∧
        ((algorithmTag = 2 ∧ length = 16) ∨
         (algorithmTag = 3 ∧ length = 32) ∨
         (algorithmTag = 4 ∧ length = 32)) := by
  by_cases typeExact : typeTag = 12
  · subst typeTag
    by_cases algorithmTwo : algorithmTag = 2
    · subst algorithmTag
      simp [digest_shape_valid]
    · by_cases algorithmThree : algorithmTag = 3
      · subst algorithmTag
        simp [digest_shape_valid]
      · by_cases algorithmFour : algorithmTag = 4
        · subst algorithmTag
          simp [digest_shape_valid]
        · simp [digest_shape_valid, algorithmTwo, algorithmThree, algorithmFour]
  · simp [digest_shape_valid, typeExact]

/-- The migration wrapper cannot alter lifecycle state. -/
theorem migration_lifecycle_wrapper_preserves_state (state : Lifecycle) :
    migrated_lifecycle state = ok state := by
  cases state <;> rfl
