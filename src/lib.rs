pub use device_driver_macro::device_driver;
pub use num_enum::TryFromPrimitive;

/// Marker trait used to check for `num_enum::TryFromPrimitive`.
pub trait FallibleField {}

impl<T> FallibleField for T where T: TryFromPrimitive {}

/// If this trait is implemented, it is guaranteed that running
/// `mem::transmute(val as u32 & MASK) -> Field` is not UB.
///
/// Requirements for implementing this type:
///
/// - Only for `enum`s
/// - The enum must have `#[repr(uXX)]`
/// - XX >= `NUM_BITS`
pub unsafe trait InfallibleField {
    /// Number of bits in the field.
    const NUM_BITS: u32;

    /// The mask for the field.
    const MASK: u32 = (1 << Self::NUM_BITS) - 1;
}

/// A bit field.
#[repr(u8)]
pub enum Bit {
    /// The bit is 0.
    _0,
    /// The bit is 1.
    _1,
}

unsafe impl InfallibleField for Bit {
    const NUM_BITS: u32 = 1;
}
