import Integrity

open Aeneas
open Aeneas.Result
open Integrity

/-- Deliberately false: a waiver cannot synthesize tested assurance. -/
theorem invalid_waiver_laundering :
    aggregate_tested false false false false true false = ok true := by
  rfl
