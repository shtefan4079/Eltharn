#![forbid(unsafe_code)]

use el_extension_api::abi::{
    EXPORT_INIT, EXPORT_MEMORY, EXPORT_SHUTDOWN, EXPORT_TICK, EXPORT_WORLD_ENTER,
    EXPORT_WORLD_EXIT, HOST_CALL_FN, HOST_MODULE,
};
use el_extension_api::services::{WASM_GUEST_FUNCTABLE_MAX, WASM_MAX_PAGES};
use wasmparser::{
    Encoding, ExternalKind, FuncType, Parser, Payload, RefType, TableType, TypeRef, ValType,
    Validator, WasmFeatures,
};

pub const MAX_MODULE_BYTES: usize = 8 * 1024 * 1024;

pub fn baseline_features() -> WasmFeatures {
    let mut f = WasmFeatures::empty();
    f.set(WasmFeatures::MUTABLE_GLOBAL, true);
    f.set(WasmFeatures::SATURATING_FLOAT_TO_INT, true);
    f.set(WasmFeatures::SIGN_EXTENSION, true);
    f.set(WasmFeatures::BULK_MEMORY, true);
    f.set(WasmFeatures::BULK_MEMORY_OPT, true);
    f.set(WasmFeatures::FLOATS, true);
    f
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionErrorKind {
    Oversized,
    NotModule,
    InvalidWasm,
    UnsupportedSection,
    BadImport,
    BadMemory,
    BadMemoryExport,
    BadTable,
    MissingExport,
    BadExport,
    HasStart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionError {
    kind: AdmissionErrorKind,
    message: String,
}

impl AdmissionError {
    fn new(kind: AdmissionErrorKind, msg: impl Into<String>) -> Self {
        Self {
            kind,
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AdmissionError {}

fn is_lifecycle_ok(ft: &FuncType) -> bool {
    ft.params().is_empty() && ft.results() == [ValType::I32]
}

fn is_host_call_ok(ft: &FuncType) -> bool {
    ft.params() == [ValType::I32; 5] && ft.results() == [ValType::I32]
}

pub fn validate(bytes: &[u8]) -> Result<(), AdmissionError> {
    if bytes.len() > MAX_MODULE_BYTES {
        return Err(AdmissionError::new(
            AdmissionErrorKind::Oversized,
            format!("{} bytes exceed 8MiB cap", bytes.len()),
        ));
    }

    if let Some(item) = Parser::new(0).parse_all(bytes).next() {
        let payload = item.map_err(|e| {
            AdmissionError::new(AdmissionErrorKind::InvalidWasm, format!("header: {e}"))
        })?;
        match payload {
            Payload::Version { num, encoding, .. } => {
                if encoding != Encoding::Module || num != 1 {
                    return Err(AdmissionError::new(
                        AdmissionErrorKind::NotModule,
                        format!("not a core module v1 (num={num}, enc={encoding:?})"),
                    ));
                }
            }
            _ => {}
        }
    }

    Validator::new_with_features(baseline_features())
        .validate_all(bytes)
        .map_err(|e| AdmissionError::new(AdmissionErrorKind::InvalidWasm, format!("{e}")))?;

    let mut types: Vec<FuncType> = Vec::new();
    let mut imports: Vec<(String, String, TypeRef)> = Vec::new();
    let mut func_type_indices: Vec<u32> = Vec::new();
    let mut memories: Vec<wasmparser::MemoryType> = Vec::new();
    let mut tables: Vec<TableType> = Vec::new();
    let mut exports: Vec<(String, ExternalKind, u32)> = Vec::new();

    for item in Parser::new(0).parse_all(bytes) {
        let payload = item.map_err(|e| {
            AdmissionError::new(AdmissionErrorKind::InvalidWasm, format!("section: {e}"))
        })?;
        match payload {
            Payload::Version { num, encoding, .. } => {
                if encoding != Encoding::Module || num != 1 {
                    return Err(AdmissionError::new(
                        AdmissionErrorKind::NotModule,
                        format!("not a core module v1 (num={num}, enc={encoding:?})"),
                    ));
                }
            }
            Payload::TypeSection(r) => {
                for t in r.into_iter_err_on_gc_types() {
                    let ft = t.map_err(|e| {
                        AdmissionError::new(
                            AdmissionErrorKind::InvalidWasm,
                            format!("gc type denied: {e}"),
                        )
                    })?;
                    types.push(ft);
                }
            }
            Payload::ImportSection(r) => {
                for imp in r.into_imports() {
                    let imp = imp.map_err(|e| {
                        AdmissionError::new(
                            AdmissionErrorKind::InvalidWasm,
                            format!("import read: {e}"),
                        )
                    })?;
                    if imports.len() >= 2 {
                        return Err(AdmissionError::new(
                            AdmissionErrorKind::BadImport,
                            "more than one import",
                        ));
                    }
                    imports.push((imp.module.to_string(), imp.name.to_string(), imp.ty));
                }
            }
            Payload::FunctionSection(r) => {
                for idx in r {
                    let idx = idx.map_err(|e| {
                        AdmissionError::new(
                            AdmissionErrorKind::InvalidWasm,
                            format!("func section: {e}"),
                        )
                    })?;
                    func_type_indices.push(idx);
                }
            }
            Payload::TableSection(r) => {
                for t in r {
                    let t = t.map_err(|e| {
                        AdmissionError::new(
                            AdmissionErrorKind::InvalidWasm,
                            format!("table section: {e}"),
                        )
                    })?;
                    if tables.len() >= 2 {
                        return Err(AdmissionError::new(
                            AdmissionErrorKind::BadTable,
                            "more than one defined table",
                        ));
                    }
                    tables.push(t.ty);
                }
            }
            Payload::MemorySection(r) => {
                for m in r {
                    let m = m.map_err(|e| {
                        AdmissionError::new(
                            AdmissionErrorKind::InvalidWasm,
                            format!("memory section: {e}"),
                        )
                    })?;
                    if memories.len() >= 2 {
                        return Err(AdmissionError::new(
                            AdmissionErrorKind::BadMemory,
                            "more than one defined memory",
                        ));
                    }
                    memories.push(m);
                }
            }
            Payload::ExportSection(r) => {
                for e in r {
                    let e = e.map_err(|e| {
                        AdmissionError::new(
                            AdmissionErrorKind::InvalidWasm,
                            format!("export section: {e}"),
                        )
                    })?;
                    exports.push((e.name.to_string(), e.kind, e.index));
                }
            }
            Payload::StartSection { .. } => {
                return Err(AdmissionError::new(
                    AdmissionErrorKind::HasStart,
                    "start section forbidden",
                ));
            }
            Payload::End(_) => {}
            Payload::GlobalSection(_)
            | Payload::ElementSection(_)
            | Payload::DataCountSection { .. }
            | Payload::DataSection(_)
            | Payload::CodeSectionStart { .. }
            | Payload::CodeSectionEntry(_)
            | Payload::CustomSection(_) => {}
            _ => {
                return Err(AdmissionError::new(
                    AdmissionErrorKind::UnsupportedSection,
                    "unsupported section",
                ));
            }
        }
    }

    if imports.len() > 1 {
        return Err(AdmissionError::new(
            AdmissionErrorKind::BadImport,
            "at most one import allowed",
        ));
    }
    let mut imported_func_count: u32 = 0;
    if let Some((module, name, ty)) = imports.first() {
        match ty {
            TypeRef::Func(type_idx) => {
                if module != HOST_MODULE || name != HOST_CALL_FN {
                    return Err(AdmissionError::new(
                        AdmissionErrorKind::BadImport,
                        "unknown import module or name",
                    ));
                }
                let ft = types.get(*type_idx as usize).ok_or_else(|| {
                    AdmissionError::new(AdmissionErrorKind::BadImport, "bad import type index")
                })?;
                if !is_host_call_ok(ft) {
                    return Err(AdmissionError::new(
                        AdmissionErrorKind::BadImport,
                        "host call must be (i32 x5)->i32",
                    ));
                }
                imported_func_count = 1;
            }
            _ => {
                return Err(AdmissionError::new(
                    AdmissionErrorKind::BadImport,
                    "non-function import denied",
                ));
            }
        }
    }

    if memories.len() != 1 {
        return Err(AdmissionError::new(
            AdmissionErrorKind::BadMemory,
            format!(
                "exactly one defined memory required, got {}",
                memories.len()
            ),
        ));
    }
    let mem = &memories[0];
    let mem_max = mem
        .maximum
        .ok_or_else(|| AdmissionError::new(AdmissionErrorKind::BadMemory, "memory max required"))?;
    if mem.memory64 || mem.shared || mem.page_size_log2.is_some() {
        return Err(AdmissionError::new(
            AdmissionErrorKind::BadMemory,
            "only unshared memory32 with 64KiB pages",
        ));
    }
    if mem_max > WASM_MAX_PAGES as u64 || mem.initial > mem_max {
        return Err(AdmissionError::new(
            AdmissionErrorKind::BadMemory,
            format!("memory limits invalid: {}..={}", mem.initial, mem_max),
        ));
    }

    if tables.len() > 1 {
        return Err(AdmissionError::new(
            AdmissionErrorKind::BadTable,
            "at most one defined table",
        ));
    }
    if let Some(t) = tables.first() {
        let tmax = t.maximum.ok_or_else(|| {
            AdmissionError::new(AdmissionErrorKind::BadTable, "table max required")
        })?;
        if t.element_type != RefType::FUNCREF || t.table64 || t.shared {
            return Err(AdmissionError::new(
                AdmissionErrorKind::BadTable,
                "only unshared funcref32 table",
            ));
        }
        if tmax > WASM_GUEST_FUNCTABLE_MAX as u64 || t.initial > tmax {
            return Err(AdmissionError::new(
                AdmissionErrorKind::BadTable,
                format!("table limits invalid: {}..={}", t.initial, tmax),
            ));
        }
    }

    let find_export = |want: &str| exports.iter().find(|(n, _, _)| n == want).cloned();
    match find_export(EXPORT_MEMORY) {
        Some((_, ExternalKind::Memory, 0)) => {}
        Some((_, kind, idx)) => {
            return Err(AdmissionError::new(
                AdmissionErrorKind::BadMemoryExport,
                format!("bad memory export kind={kind:?} idx={idx}"),
            ));
        }
        None => {
            return Err(AdmissionError::new(
                AdmissionErrorKind::BadMemoryExport,
                "missing memory export",
            ));
        }
    }

    let resolve_defined = |name: &str| -> Result<Option<FuncType>, AdmissionError> {
        let Some((_, kind, idx)) = find_export(name) else {
            return Ok(None);
        };
        if kind != ExternalKind::Func {
            return Err(AdmissionError::new(
                AdmissionErrorKind::BadExport,
                format!("{name} must be a function export"),
            ));
        }
        if idx < imported_func_count {
            return Err(AdmissionError::new(
                AdmissionErrorKind::BadExport,
                format!("{name} points at host import"),
            ));
        }
        let defined = idx - imported_func_count;
        let type_idx = *func_type_indices.get(defined as usize).ok_or_else(|| {
            AdmissionError::new(AdmissionErrorKind::BadExport, format!("{name} index OOB"))
        })?;
        let ft = types.get(type_idx as usize).ok_or_else(|| {
            AdmissionError::new(AdmissionErrorKind::BadExport, format!("{name} type OOB"))
        })?;
        Ok(Some(ft.clone()))
    };

    for required in [EXPORT_INIT, EXPORT_TICK, EXPORT_SHUTDOWN] {
        match resolve_defined(required)? {
            None => {
                return Err(AdmissionError::new(
                    AdmissionErrorKind::MissingExport,
                    format!("missing {required}"),
                ));
            }
            Some(ft) if !is_lifecycle_ok(&ft) => {
                return Err(AdmissionError::new(
                    AdmissionErrorKind::BadExport,
                    format!("{required} must be ()->i32"),
                ));
            }
            _ => {}
        }
    }
    for optional in [EXPORT_WORLD_ENTER, EXPORT_WORLD_EXIT] {
        if let Some(ft) = resolve_defined(optional)? {
            if !is_lifecycle_ok(&ft) {
                return Err(AdmissionError::new(
                    AdmissionErrorKind::BadExport,
                    format!("{optional} must be ()->i32"),
                ));
            }
        }
    }

    Ok(())
}
