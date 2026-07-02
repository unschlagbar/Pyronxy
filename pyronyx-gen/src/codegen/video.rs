use super::{Writer, file_header};
use crate::codegen::rust_name;
use crate::codegen::types::{write_funcpointer, write_handle, write_struct, write_union};
use crate::parse::c_to_rust;
use crate::parse::registry::Registry;
use crate::parse::types::TypeKind;
use std::collections::HashSet;

pub fn generate(registry: &Registry, out_path: &str, lifetimes: &HashSet<String>) {
    let mut w = Writer::new();
    file_header(&mut w, "vk/types.rs");

    w.ln("#![allow(non_camel_case_types)]");
    w.ln("#![allow(non_upper_case_globals)]");
    w.blank();
    w.ln("use super::enums::*;");
    w.ln("use core::{mem, ptr};");
    w.ln("use core::marker::PhantomData;");
    w.blank();

    // ── Constants ─────────────────────────────────────────────────────────
    if !registry.constants.is_empty() {
        w.ln("// ── Constants ───────────────────────────────────────────────");
        for c in &registry.constants {
            if let Some(comment) = &c.comment {
                w.ln(&format!("/// {comment}"));
            }
            w.ln(&format!("pub const {}: {} = {};", c.name, c.ty, c.value));
        }
        w.blank();
    }

    for (_, ty) in &registry.types {
        match &ty.kind {
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
                write_struct(
                    &mut w,
                    &rust_name(&ty.name),
                    members,
                    extends,
                    *returned_only,
                    &registry.stypes,
                    *lifetime,
                    lifetimes,
                    false,
                );
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
