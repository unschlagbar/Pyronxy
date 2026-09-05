# Changelog

## [0.4.0]

### Updated to Vulkan 1.4.357.1

### Changes

Added some missing  `_len()` companion fns

## [0.3.2]

### Fix

C bitfield members (`:24`/`:8`/`:1`) were generated as full `u32` fields, breaking the memory layout of every struct using them (`vk::AccelerationStructureInstanceKHR` was 72 instead of 64 bytes, the StdVideo `*Flags` structs were up to 32x too big). All generated struct sizes are now verified against the C headers.

### Changes

Bitfield groups are merged into a single member of the new packed types `Packed24_8`, `Packed24_5_3` and `Packed9_9_6_4_4` (ash-style), e.g. `AccelerationStructureInstanceKHR::instance_custom_index_and_mask: Packed24_8`
StdVideo `*Flags` structs are now vk-style bitflag newtypes with one constant per bit (e.g. `H264SpsVuiFlags::VideoFullRangeFlag`) including Display/Debug by flag name
`H265HrdFlags` is the one exception with multi-bit fields; it packs them into a `bitfields: u32` member with getter/setter methods

## [0.3.1]

### Changes

Surface creation now supports MacOS

## [0.3.0]

### Changes

`Device::acquire_next_image` now returns `Result<Suboptimal<u32>>` to allow for suboptimal checking

## [0.2.5]

### Updated to Vulkan 1.4.350.0

Added 7 new extensions

## [0.2.4]

### Added

Default & null() to PhysicalDevice
Implemented `From<Extent2D> for Rect2D`

### Changes

All method params that where `[c_void]` are now `[u8]`

## [0.2.3]

### Added

vk::Result type as wrapper for Result<T, vk::Error>
vk::Error now implements std::error::Error;


## [0.2.2]

### Added

Missing len functions.

### Fix

Builing on systems where c_char = u8, Raspberry pi in my case

## [0.2.1]

### Added Error Messages for not loaded functions

Not loaded core functions throw `CORE_LOAD_ERROR` on use.
Not loaded extension functions throw `EXT_LOAD_ERROR` on use.

### Fix

Some inconstistent function docs
