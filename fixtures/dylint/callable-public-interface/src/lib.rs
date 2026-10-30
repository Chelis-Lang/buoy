#![no_std]
#![doc = "Negative fixture: a functional core exposing caller-provided behavior."]

/// Violates the boundary with a direct function-pointer parameter.
pub fn with_function_pointer(value: u8, transform: fn(u8) -> u8) -> u8 {
    transform(value)
}

/// Violates the boundary with a generic `Fn` callback bound.
pub fn with_callback<F>(value: u8, transform: F) -> u8
where
    F: Fn(u8) -> u8,
{
    transform(value)
}

/// Violates the boundary with a generic `FnMut` callback bound.
pub fn with_mut_callback<F>(value: u8, mut transform: F) -> u8
where
    F: FnMut(u8) -> u8,
{
    transform(value)
}

/// Violates the boundary with a generic `FnOnce` callback bound.
pub fn with_once_callback<F>(value: u8, transform: F) -> u8
where
    F: FnOnce(u8) -> u8,
{
    transform(value)
}

/// Violates the boundary with a callable trait object.
pub fn with_trait_object(value: u8, transform: &dyn Fn(u8) -> u8) -> u8 {
    transform(value)
}

/// Violates the boundary with an opaque callable parameter.
pub fn with_opaque_callback(value: u8, transform: impl Fn(u8) -> u8) -> u8 {
    transform(value)
}
