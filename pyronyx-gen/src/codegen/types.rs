use std::collections::HashSet;

use indexmap::IndexMap;

use heck::{ToSnakeCase, ToUpperCamelCase};

use super::{Writer, file_header};
use crate::codegen::{
    COPY, LATEXMATH_LEN_MAP, PACKED_FIELD_NAMES, PACKED_TYPES, a_lifetime, const_name, rust_member,
    rust_name, u_lifetime,
};
use crate::parse::c_to_rust;
use crate::parse::commands::Param;
use crate::parse::registry::Registry;
use crate::parse::types::{Member, TypeKind};

pub fn generate(registry: &Registry, out_path: &str, lifetimes: &HashSet<String>) {
    let mut w = Writer::new();
    file_header(&mut w, "vk/types.rs");

    w.ln("#![allow(non_camel_case_types)]");
    w.blank();
    w.ln("use crate::video::*;");
    w.ln("use super::enums::*;");
    w.ln("use super::packed::*;");
    w.ln("use super::platform_types::*;");
    w.ln("use super::bitflags::*;");
    w.ln("use super::constants::*;");
    w.ln("use core::ffi::{c_void, c_char, c_int};");
    w.ln("use core::{mem, ptr, fmt};");
    w.ln("use core::marker::PhantomData;");
    w.blank();

    let mut seen_aliases = HashSet::new();

    for ty in registry.types.values() {
        match &ty.kind {
            TypeKind::Alias { lifetime, alias } => {
                let name = rust_name(&ty.name);
                if seen_aliases.insert(name.clone()) {
                    let lifetime = a_lifetime(*lifetime);
                    w.ln(&format!(
                        "pub type {}{lifetime} = {}{lifetime};",
                        name,
                        rust_name(alias)
                    ));
                }
            }
            TypeKind::BaseType { inner } => {
                let rust = c_to_rust(inner);
                w.ln(&format!("pub type {} = {};", rust_name(&ty.name), rust));
            }

            TypeKind::Handle { dispatchable, .. } => {
                write_handle(&mut w, &rust_name(&ty.name), *dispatchable);
                w.blank();
            }

            TypeKind::Struct {
                members,
                extends,
                returned_only,
                lifetime,
            } => {
                let name = rust_name(&ty.name);

                write_struct(
                    &mut w,
                    &name,
                    members,
                    extends,
                    *returned_only,
                    &registry.stypes,
                    *lifetime,
                    lifetimes,
                    false,
                );

                if let Some(mutable) = registry.next_chains.get(&ty.name) {
                    next_chain(&mut w, &name, *mutable);
                }

                w.blank();
            }

            TypeKind::Union { members, lifetime } => {
                write_union(&mut w, &rust_name(&ty.name), members, *lifetime, lifetimes);
                w.blank();
            }

            TypeKind::FuncPointer {
                return_type,
                params,
            } => {
                write_funcpointer(&mut w, &ty.name, return_type, params);
                w.blank();
            }
            _ => (),
        }
    }

    w.save(out_path);
}

pub fn write_handle(w: &mut Writer, name: &str, dispatchable: bool) {
    if dispatchable {
        w.ln("#[repr(transparent)]");
        w.ln("#[derive(Debug, Default)]");
        w.ln(&format!("pub struct {name}(*mut u8);"));
        w.ln(&format!("impl {name} {{"));
        w.ln("#[inline] pub const fn null() -> Self { Self(ptr::null_mut()) }");
        w.ln("#[inline] pub const fn is_null(self) -> bool { self.0.is_null() }");
        w.ln("}");
        w.ln(&format!("impl Copy  for {name} {{}}"));
        w.ln(&format!(
            "impl Clone for {name} {{ fn clone(&self) -> Self {{ *self }} }}"
        ));
        w.ln(&format!("unsafe impl Send for {name} {{}}"));
        w.ln(&format!("unsafe impl Sync for {name} {{}}"));
    } else {
        w.ln("#[repr(transparent)]");
        w.ln("#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]");
        w.ln(&format!("pub struct {name}(pub u64);"));
        w.ln(&format!("impl {name} {{"));
        w.ln("#[inline] pub const fn null() -> Self { Self(0) }");
        w.ln("#[inline] pub const fn is_null(self) -> bool { self.0 == 0 }");
        w.ln("}");
        w.ln(&format!(
            "impl fmt::Debug for {name} {{\
             fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result \
             {{ write!(f, \"{name}(0x{{:x}})\", self.0) }} }}"
        ));
        w.ln(&format!(
            "impl fmt::Pointer for {name} {{\
             fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result \
             {{ fmt::Debug::fmt(&self, f) }} }}"
        ));
    }
}

pub fn write_struct(
    w: &mut Writer,
    name: &str,
    members: &[Member],
    extends: &Option<String>,
    returned_only: bool,
    stypes: &IndexMap<String, (String, bool)>,
    lifetime: bool,
    lifetimes: &HashSet<String>,
    derive_default: bool,
) {
    if try_write_flag_struct(w, name, members) {
        return;
    }

    let (members, bit_accessors) = merge_bitfields(name, members);
    let members = &members[..];

    let next_chains = if let Some(ext) = extends {
        let next_chains = ext.split(",").collect::<Vec<_>>();
        let ext = next_chains
            .iter()
            .map(|v| format!("`{}`", rust_name(v)))
            .collect::<Vec<_>>()
            .join(", ");

        w.ln(&format!("/// Extends: {ext}"));
        next_chains
    } else {
        Vec::new()
    };

    if returned_only {
        w.ln("/// returned_only");
    }

    let default_derive = if derive_default { ", Default" } else { "" };

    let eq_derive = if COPY.contains(&name) {
        ", Hash, PartialEq, Eq"
    } else {
        ""
    };
    w.ln("#[repr(C)]");
    w.ln(&format!(
        "#[derive(Copy, Clone, Debug{eq_derive}{default_derive})]"
    ));

    w.ln(&format!("pub struct {name}{} {{", a_lifetime(lifetime)));

    let mut ps = HashSet::new();

    for m in members {
        let m_lifetime = lifetimes.contains(&rust_name(&m.ty));
        let rust_ty = member_rust_type(m, m_lifetime);

        let final_ty = build_array_type(&rust_ty, &m.array_dims);

        if !m.comment.is_empty() {
            w.ln(&format!("/// {}", m.comment));
        }
        if m.optional && m.pointer_depth > 0 {
            w.ln("/// Nullable");
        }
        if let Some(len) = &m.len {
            let readable = resolve_len(len);
            w.ln(&format!("/// Len: {readable}"));
        }

        w.ln(&format!(
            "pub {}: {},",
            rust_member(&m.name, &mut ps),
            final_ty
        ));
    }

    if lifetime {
        w.ln("pub _marker: PhantomData<&'a ()>,");
    }
    w.ln("}");

    if !derive_default {
        write_default_impl(w, name, members, stypes, lifetime);
    }

    if !bit_accessors.is_empty() {
        write_bitfield_accessors(w, name, lifetime, &bit_accessors);
    }

    for next_chain in next_chains {
        w.ln(&format!(
            "impl Extends{} for {name}<'_> {{}}",
            rust_name(next_chain)
        ));
    }
}

/// Structs made up entirely of 1-bit C bitfields (the StdVideo `*Flags` structs)
/// become vk-style bitflag newtypes with one constant per bit instead of a
/// struct with an accessor pair per bit. Structs with wider bitfields (e.g.
/// `StdVideoH265HrdFlags`) fall through to `merge_bitfields`.
fn try_write_flag_struct(w: &mut Writer, name: &str, members: &[Member]) -> bool {
    let is_flag_run = !members.is_empty()
        && members.iter().all(|m| m.bits.is_some())
        && members.iter().map(|m| m.bits.unwrap()).sum::<u32>() <= 32
        && members
            .iter()
            .all(|m| m.name.starts_with("reserved") || m.bits == Some(1))
        && members.iter().any(|m| !m.name.starts_with("reserved"));

    if !is_flag_run {
        return false;
    }

    let mut flags = Vec::new();
    let mut offset = 0;
    for m in members {
        if !m.name.starts_with("reserved") {
            flags.push((m.name.to_upper_camel_case(), offset, m.comment.clone()));
        }
        offset += m.bits.unwrap();
    }

    w.ln("#[repr(transparent)]");
    w.ln("#[derive(Clone, Copy, Hash, PartialEq, Eq)]");
    w.ln(&format!("pub struct {name}(pub(crate) u32);"));
    w.ln(&format!("crate::vk_bitflags_wrapped!({name}, u32);"));

    w.ln(&format!("impl {name} {{"));
    for (flag, offset, comment) in &flags {
        if !comment.is_empty() {
            w.ln(&format!("/// {comment}"));
        }
        w.ln(&format!("pub const {flag}: Self = Self(1 << {offset});"));
    }
    w.ln("}");

    let bits = flags
        .iter()
        .map(|(flag, _, _)| format!("({name}::{flag}, \"{flag}\")"))
        .collect::<Vec<_>>()
        .join(", ");

    w.ln(&format!(
        "impl core::fmt::Display for {name} {{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        const BITS: &[({name}, &str)] = &[{bits}];

        let mut first = true;
        for &(flag, name) in BITS {{
            if self.contains(flag) {{
                if !first {{
                    f.write_str(\" | \")?;
                }}
                f.write_str(name)?;
                first = false;
            }}
        }}
        if first {{
            f.write_str(\"0\")?;
        }}
        Ok(())
    }}
}}
impl core::fmt::Debug for {name} {{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        core::fmt::Display::fmt(self, f)
    }}
}}"
    ));

    true
}

/// A bitfield packed into a generated `u32` storage member, exposed through
/// getter/setter methods instead of a field.
struct PackedBitfield {
    storage: String,
    name: String,
    offset: u32,
    width: u32,
}

/// Bitfield ordering in C is implementation-defined, so the registry structs using
/// bitfields (`:24`/`:1` etc.) define a normative packed layout instead (LSB first).
/// Merge each run of bitfield members occupying one u32 into a single member:
/// layouts listed in `PACKED_TYPES` become a hand-written packed type
/// (`pyronyx/src/vk/packed.rs`), everything else becomes a plain `u32` storage
/// member with generated bit accessors.
fn merge_bitfields(struct_name: &str, members: &[Member]) -> (Vec<Member>, Vec<PackedBitfield>) {
    if members.iter().all(|m| m.bits.is_none()) {
        return (members.to_vec(), Vec::new());
    }

    let mut out = Vec::with_capacity(members.len());
    let mut accessors = Vec::new();
    let mut unit: Vec<&Member> = Vec::new();
    let mut used = 0;
    let mut unit_count = 0;

    let mut close_unit = |unit: &mut Vec<&Member>,
                          used: &mut u32,
                          out: &mut Vec<Member>,
                          accessors: &mut Vec<PackedBitfield>| {
        out.push(merge_unit(struct_name, unit, unit_count, accessors));
        unit_count += 1;
        unit.clear();
        *used = 0;
    };

    for m in members {
        if let Some(bits) = m.bits {
            // C starts a new storage unit when the next bitfield does not fit.
            if used + bits > 32 {
                close_unit(&mut unit, &mut used, &mut out, &mut accessors);
            }
            unit.push(m);
            used += bits;
            if used == 32 {
                close_unit(&mut unit, &mut used, &mut out, &mut accessors);
            }
        } else {
            if !unit.is_empty() {
                close_unit(&mut unit, &mut used, &mut out, &mut accessors);
            }
            out.push(m.clone());
        }
    }
    if !unit.is_empty() {
        close_unit(&mut unit, &mut used, &mut out, &mut accessors);
    }

    (out, accessors)
}

fn merge_unit(
    struct_name: &str,
    unit: &[&Member],
    unit_index: u32,
    accessors: &mut Vec<PackedBitfield>,
) -> Member {
    let widths: Vec<u32> = unit.iter().map(|m| m.bits.unwrap()).collect();

    let (name, ty, comment) = if let Some(&(_, packed_ty)) =
        PACKED_TYPES.iter().find(|(w, _)| *w == widths.as_slice())
    {
        // `reserved` members stay out of the name unless dropping them leaves fewer than two.
        let mut names: Vec<String> = unit
            .iter()
            .filter(|m| m.name != "reserved")
            .map(|m| m.name.to_snake_case())
            .collect();
        if names.len() < 2 {
            names = unit.iter().map(|m| m.name.to_snake_case()).collect();
        }

        let mut name = names.join("_and_");
        if let Some(&(_, replacement)) = PACKED_FIELD_NAMES.iter().find(|&&(k, _)| k == name) {
            name = replacement.to_string();
        }

        let layout = unit
            .iter()
            .map(|m| format!("`{}`: {}", m.name.to_snake_case(), m.bits.unwrap()))
            .collect::<Vec<_>>()
            .join(", ");

        (
            name,
            packed_ty.to_string(),
            format!("Bitfields (LSB to MSB) {layout} - construct with [`{packed_ty}::new`]"),
        )
    } else {
        if widths.iter().sum::<u32>() == 32 && !unit.iter().any(|m| m.name.starts_with("reserved"))
        {
            println!(
                "[codegen] bitfield layout {widths:?} in {struct_name} fills a u32 - \
                 consider adding a packed type to PACKED_TYPES and pyronyx/src/vk/packed.rs"
            );
        }

        let name = if unit_index == 0 {
            "bitfields".to_string()
        } else {
            format!("bitfields{unit_index}")
        };

        let mut offset = 0;
        for m in unit {
            if !m.name.starts_with("reserved") {
                accessors.push(PackedBitfield {
                    storage: name.clone(),
                    name: m.name.to_snake_case(),
                    offset,
                    width: m.bits.unwrap(),
                });
            }
            offset += m.bits.unwrap();
        }

        (
            name,
            "u32".to_string(),
            "Packed bitfields - use the generated getter/setter methods".to_string(),
        )
    };

    Member {
        comment,
        full_ty: ty.clone(),
        ty,
        name,
        optional: false,
        len: None,
        is_const: false,
        pointer_depth: 0,
        array_dims: Vec::new(),
        bits: None,
    }
}

fn write_bitfield_accessors(
    w: &mut Writer,
    name: &str,
    lifetime: bool,
    accessors: &[PackedBitfield],
) {
    w.ln(&format!("impl {name}{} {{", u_lifetime(lifetime)));

    for a in accessors {
        let PackedBitfield {
            storage,
            name,
            offset,
            width,
        } = a;

        if *width == 1 {
            w.ln(&format!(
                "#[inline]
pub const fn {name}(&self) -> bool {{
    self.{storage} & (1 << {offset}) != 0
}}
#[inline]
pub const fn set_{name}(&mut self, value: bool) {{
    self.{storage} = (self.{storage} & !(1 << {offset})) | ((value as u32) << {offset});
}}"
            ));
        } else {
            let mask = (1u32 << width) - 1;
            w.ln(&format!(
                "/// Only the low {width} bits are used.
#[inline]
pub const fn {name}(&self) -> u32 {{
    (self.{storage} >> {offset}) & {mask:#x}
}}
/// Only the low {width} bits are used.
#[inline]
pub const fn set_{name}(&mut self, value: u32) {{
    self.{storage} = (self.{storage} & !({mask:#x} << {offset})) | ((value & {mask:#x}) << {offset});
}}"
            ));
        }
    }

    w.ln("}");
}

fn write_default_impl(
    w: &mut Writer,
    struct_name: &str,
    members: &[Member],
    stypes: &IndexMap<String, (String, bool)>,
    lifetime: bool,
) {
    w.ln(&format!(
        "impl Default for {struct_name}{} {{",
        u_lifetime(lifetime)
    ));
    w.ln("fn default() -> Self {");
    w.ln("Self {");

    let mut ps = HashSet::new();

    for m in members {
        let value = default_value_for_member(m, struct_name, stypes);
        w.ln(&format!("{}: {},", rust_member(&m.name, &mut ps), value));
    }

    if lifetime {
        w.ln("_marker: PhantomData,");
    }

    w.ln("}}}");
    w.blank();
}

fn default_value_for_member(
    m: &Member,
    struct_name: &str,
    stypes: &IndexMap<String, (String, bool)>,
) -> String {
    // 1. Pointers → null
    if m.pointer_depth > 0 {
        return "ptr::null_mut()".to_string();
    }

    // 2. Arrays (any size: [6], [3][4], [VK_UUID_SIZE], [256] …) → always zeroed
    //    This is the only way that works for ALL array sizes and ALL Rust versions.
    if !m.array_dims.is_empty() {
        return "unsafe { mem::zeroed() }".to_string();
    }

    // 3. Special case for sType
    if m.name == "sType" {
        if stypes
            .values()
            .find(|&v| rust_name(&v.0) == struct_name && v.1)
            .is_some()
        {
            return format!("StructureType::{struct_name}");
        } else {
            return "unsafe { mem::zeroed() }".to_string();
        }
    }

    // 4. Everything else
    default_value_for_type(&m.ty, &m.full_ty)
}

fn default_value_for_type(ty: &str, _full_ty: &str) -> String {
    match ty {
        "f32" | "f64" => "0.0".to_string(),
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "usize" => "0".to_string(),
        "VkBool32" => "FALSE".to_string(),
        _ => "Default::default()".to_string(),
    }
}

pub fn write_union(
    w: &mut Writer,
    name: &str,
    members: &[Member],
    lifetime: bool,
    lifetimes: &HashSet<String>,
) {
    let u_lifetime = u_lifetime(lifetime);
    let lifetime = a_lifetime(lifetime);

    w.ln("#[repr(C)]");
    w.ln("#[derive(Copy, Clone)]");
    w.ln(&format!("pub union {name}{lifetime} {{"));

    let mut ps = HashSet::new();

    for m in members {
        let m_lifetime = lifetimes.contains(&rust_name(&m.ty));
        let rust_ty = member_rust_type(m, m_lifetime);

        let final_ty = build_array_type(&rust_ty, &m.array_dims);

        if !m.comment.is_empty() {
            w.ln(&format!("/// {}", m.comment));
        }
        if m.optional && m.pointer_depth > 0 {
            w.ln("/// Nullable");
        }

        w.ln(&format!(
            "pub {}: {},",
            rust_member(&m.name, &mut ps),
            final_ty
        ));
    }
    w.ln("}");

    w.ln(&format!("impl Default for {name}{u_lifetime} {{"));
    w.ln("fn default() -> Self {");
    w.ln("unsafe { mem::zeroed() }");
    w.ln("}");
    w.ln("}");
    w.ln(&format!(
        r#"
impl fmt::Debug for {name}{u_lifetime} {{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {{
        fmt.debug_struct("{name}").finish()
    }}
    }}"#
    ));
}

pub fn write_funcpointer(w: &mut Writer, name: &str, return_type: &str, params: &[Param]) {
    let ret = rust_name(return_type);

    let mut ps = HashSet::new();

    let args = params
        .iter()
        .map(|p| rust_member(&p.name, &mut ps) + ": " + &rust_name(&p.ty))
        .collect::<Vec<_>>()
        .join(", ");

    let ret = if ret == "c_void" {
        String::new()
    } else {
        format!("-> {ret}")
    };

    w.ln(&format!(
        "pub type {name} = Option<extern \"system\" fn({args}) {ret}>;"
    ));
}

fn member_rust_type(m: &Member, lifetime: bool) -> String {
    if let Some(arr) = parse_array(&m.full_ty, &m.ty) {
        return arr;
    }

    let base = rust_name(&m.ty);
    let lifetime = a_lifetime(lifetime);

    match m.pointer_depth {
        0 => format!("{base}{lifetime}"),
        1 => {
            if m.is_const {
                format!("*const {base}{lifetime}")
            } else {
                format!("*mut {base}{lifetime}")
            }
        }
        2 => {
            if m.is_const {
                format!("*const *const {base}{lifetime}")
            } else {
                format!("*mut *mut {base}{lifetime}")
            }
        }
        _ => format!("*mut {base}{lifetime}"),
    }
}

fn parse_array(full_ty: &str, base_ty: &str) -> Option<String> {
    let start = full_ty.find('[')?;
    let end = full_ty.rfind(']')?;
    let size = &full_ty[start + 1..end];
    let base = c_to_rust(base_ty);

    let rust_size = const_name(size);

    Some(format!("[{base}; {rust_size}]"))
}

fn build_array_type(base_ty: &str, dims: &[String]) -> String {
    if dims.is_empty() {
        base_ty.to_string()
    } else {
        let mut ty = base_ty.to_string();
        // [3][4] → [[T; 4]; 3]
        for dim in dims.iter().rev() {
            let dim_expr = const_name(dim);
            let convert = !dim_expr.chars().all(|c| c.is_ascii_digit());
            if convert {
                ty = format!("[{}; {} as usize]", ty, dim_expr);
            } else {
                ty = format!("[{}; {}]", ty, dim_expr);
            }
        }
        ty
    }
}

pub fn resolve_len(input: &str) -> String {
    // Known latexmath expressions → explicit mapping (no backticks, it's a math expression)
    if input.starts_with("latexmath:[") {
        if let Some(&(_, rust_expr)) = LATEXMATH_LEN_MAP.iter().find(|&&(latex, _)| latex == input)
        {
            return rust_expr.to_string();
        }

        println!("[codegen] unknown latexmath len expression - add it to LATEXMATH_LEN_MAP:");
        println!("  (\"{input}\", \"<your_rust_expression_here>\"),");

        return input.to_string();
    }

    // Plain member reference(s) → snake_case wrapped in backticks so it reads as a field
    let mut ps = HashSet::new();
    input
        .split(',')
        .map(|part| {
            let part = part.trim();
            if part == "null-terminated" {
                part.to_string() // not a member, no backticks
            } else {
                format!("`{}`", rust_member(part, &mut ps)) // looks like a field reference
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn next_chain(w: &mut Writer, name: &str, _mutable: bool) {
    let trait_name = format!("Extends{name}");

    w.ln(&format!("pub trait {trait_name} {{}}"));

    w.ln(&format!("impl<'a> {name}<'a> {{"));

    w.ln("/// Inserts the given extension struct between the root and the first pointer.");
    w.ln("/// If the chain looks like `a -> b -> c`, and you call `a.next(&mut d)`,");
    w.ln("/// then the chain will look like `a -> d -> b -> c`.");
    w.ln(&format!(
        "#[inline]
pub fn next<T: {trait_name}>(mut self, next: &'a mut T) -> Self {{
    unsafe {{
        let next_base: *mut BaseOutStructure = ptr::from_mut(next).cast();

        debug_assert!(
            (*next_base).next.is_null(),
            \"next of inserted struct must be null (already in a chain?)\"
        );

        (*next_base).next = self.next as _;
        self.next = ptr::from_mut(next).cast();
    }}

    self
}}"
    ));

    w.ln("}");
}
