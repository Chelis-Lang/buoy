/- Minimal, spike-scoped model of the Aeneas Lean runtime surface used by the
committed generated file. This is a bound external model, not the full Aeneas
backend and not evidence for unsupported generated constructs. -/
macro:max value:term:max noWs "#u8" : term => `(UInt8.ofNat $value)

namespace Aeneas

inductive Result (α : Type u) where
  | ok (value : α)
  | fail
  deriving BEq, Repr

export Result (ok)

instance : Monad Result where
  pure := Result.ok
  bind result next :=
    match result with
    | Result.ok value => next value
    | Result.fail => Result.fail

namespace Std
abbrev U8 := UInt8
abbrev U64 := UInt64
end Std

namespace ControlFlow
end ControlFlow

namespace Error
end Error

end Aeneas
