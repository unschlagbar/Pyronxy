//! Packed bitfield carriers for structs whose C definition uses bitfields.
//!
//! Bitfield ordering in C is implementation-defined, so the Vulkan
//! specification defines a normative packed layout for these members instead.
//! Each type here packs one such group of bitfields into a single `u32`,
//! low bits first.

/// Holds 24 bits in the least significant bits and 8 bits in the most
/// significant bits of a single `u32`.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Packed24_8(u32);

impl Packed24_8 {
    #[inline]
    pub const fn new(low_24: u32, high_8: u8) -> Self {
        Self((low_24 & 0x00ff_ffff) | ((high_8 as u32) << 24))
    }

    /// The least significant 24 bits.
    #[inline]
    pub const fn low_24(self) -> u32 {
        self.0 & 0x00ff_ffff
    }

    /// The most significant 8 bits.
    #[inline]
    pub const fn high_8(self) -> u8 {
        (self.0 >> 24) as u8
    }
}

/// Holds 24 bits in the least significant bits and 3 bits in the most
/// significant bits of a single `u32`. The 5 bits in between are reserved
/// and held at zero.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Packed24_5_3(u32);

impl Packed24_5_3 {
    #[inline]
    pub const fn new(low_24: u32, high_3: u8) -> Self {
        Self((low_24 & 0x00ff_ffff) | (((high_3 as u32) & 0x7) << 29))
    }

    /// The least significant 24 bits.
    #[inline]
    pub const fn low_24(self) -> u32 {
        self.0 & 0x00ff_ffff
    }

    /// The most significant 3 bits.
    #[inline]
    pub const fn high_3(self) -> u8 {
        (self.0 >> 29) as u8
    }
}

/// Packs five fields of width 9, 9, 6, 4 and 4 bits (LSB to MSB) into a
/// single `u32`.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Packed9_9_6_4_4(u32);

impl Packed9_9_6_4_4 {
    #[inline]
    pub const fn new(
        low_9: u32,
        lower_mid_9: u32,
        mid_6: u32,
        upper_mid_4: u32,
        high_4: u32,
    ) -> Self {
        Self(
            (low_9 & 0x1ff)
                | ((lower_mid_9 & 0x1ff) << 9)
                | ((mid_6 & 0x3f) << 18)
                | ((upper_mid_4 & 0xf) << 24)
                | ((high_4 & 0xf) << 28),
        )
    }

    /// Bits 0..9.
    #[inline]
    pub const fn low_9(self) -> u32 {
        self.0 & 0x1ff
    }

    /// Bits 9..18.
    #[inline]
    pub const fn lower_mid_9(self) -> u32 {
        (self.0 >> 9) & 0x1ff
    }

    /// Bits 18..24.
    #[inline]
    pub const fn mid_6(self) -> u32 {
        (self.0 >> 18) & 0x3f
    }

    /// Bits 24..28.
    #[inline]
    pub const fn upper_mid_4(self) -> u32 {
        (self.0 >> 24) & 0xf
    }

    /// Bits 28..32.
    #[inline]
    pub const fn high_4(self) -> u32 {
        self.0 >> 28
    }
}
