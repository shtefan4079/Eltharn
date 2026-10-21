use crate::script::ScriptBundle;
use crate::session::{Definition, ModuleCode};
pub use el_extension_api::hooks::{HookBinding, HookMode, HookModule, HookPoint};
use rhai::packages::{
    ArithmeticPackage, BasicArrayPackage, BasicIteratorPackage, BasicMapPackage, BasicMathPackage,
    BasicStringPackage, BitFieldPackage, LogicPackage, MoreStringPackage, Package,
};
use rhai::{
    AST, ASTNode, Array, CallFnOptions, Dynamic, Engine, Expr, Map, OptimizationLevel, Scope, Stmt,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
pub const MAX_FRAME_OPERATIONS: u64 = 50_000;
pub const MAX_FRAME_CALLS: u64 = 128;
pub const MAX_HOOK_OPERATIONS: u64 = 8_000;
pub const MAX_TOOLTIP_HOOK_OPERATIONS: u64 = 16_000;
const MAX_NODES: usize = 256;
const MAX_DATA_BYTES: usize = 4096;
const MAX_DIAGNOSTICS: usize = 32;
fn engine() -> Engine {
    let mut engine = Engine::new_raw();
    ArithmeticPackage::new().register_into_engine(&mut engine);
    BasicArrayPackage::new().register_into_engine(&mut engine);
    BasicIteratorPackage::new().register_into_engine(&mut engine);
    BasicMapPackage::new().register_into_engine(&mut engine);
    BasicMathPackage::new().register_into_engine(&mut engine);
    BasicStringPackage::new().register_into_engine(&mut engine);
    BitFieldPackage::new().register_into_engine(&mut engine);
    LogicPackage::new().register_into_engine(&mut engine);
    MoreStringPackage::new().register_into_engine(&mut engine);
    engine.set_optimization_level(OptimizationLevel::None);
    engine.set_max_operations(MAX_HOOK_OPERATIONS);
    engine.set_max_call_levels(8);
    engine.set_max_expr_depths(24, 24);
    engine.set_max_variables(64);
    engine.set_max_functions(128);
    engine.set_max_modules(0);
    engine.set_max_string_size(256);
    engine.set_max_array_size(512);
    engine.set_max_map_size(128);
    engine.set_max_strings_interned(512);
    for symbol in ["eval", "print", "debug", "Fn"] {
        engine.disable_symbol(symbol);
    }
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine
}
fn compile(bundle: &ScriptBundle, module: &HookModule, engine: &Engine) -> Result<AST, String> {
    module.validate()?;
    if module.script == bundle.entry {
        return Err("Hook functions require a separate source from lifecycle entry".into());
    }
    let source = bundle
        .sources
        .get(&module.script)
        .ok_or("Hook source is absent from package")?;
    let mut ast = engine
        .compile(source)
        .map_err(|e| format!("{}: {e}", module.script))?;
    ast.set_source(&module.script);
    if !ast.statements().is_empty() {
        return Err("Hook source may contain function declarations only".into());
    }
    let mut forbidden = None;
    ast.walk(&mut |path| {
        match path.last() {
            Some(ASTNode::Stmt(Stmt::Import(..))) => forbidden = Some("imports"),
            Some(ASTNode::Expr(Expr::ThisPtr(..))) => forbidden = Some("this"),
            _ => {}
        }
        forbidden.is_none()
    });
    if let Some(kind) = forbidden {
        return Err(format!("Pure hook functions do not support {kind}"));
    }
    for binding in &module.bindings {
        if !ast.iter_functions().any(|f| {
            f.name == binding.function && f.params.len() == 1 && f.access == rhai::FnAccess::Public
        }) {
            return Err(format!(
                "Hook {} requires a public one-argument function",
                binding.function
            ));
        }
    }
    Ok(ast)
}
pub fn validate_bundle(bundle: &ScriptBundle) -> Result<(), String> {
    if let Some(module) = &bundle.hooks {
        compile(bundle, module, &engine())?;
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookDiagnostic {
    pub owner: String,
    pub point: HookPoint,
    pub function: String,
    pub error: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookOutcome {
    pub context: Value,
    pub cancelled: bool,
    pub diagnostics: Vec<HookDiagnostic>,
    pub operations: u64,
}
struct Program {
    engine: Engine,
    ast: AST,
    operations: Arc<AtomicU64>,
    operation_limit: Arc<AtomicU64>,
    bindings: Vec<HookBinding>,
}
struct Owner {
    key: String,
    dependencies: Vec<usize>,
    disabled: bool,
    program: Option<Program>,
}
pub struct HookDispatcher {
    owners: Vec<Owner>,
    snapshots: BTreeMap<String, Value>,
    pending_snapshots: BTreeMap<String, Value>,
    frame: Option<u64>,
    calls: u64,
    operations: u64,
    diagnostics: Vec<HookDiagnostic>,
}
impl HookDispatcher {
    pub fn prepare(definitions: &[Definition]) -> Result<Self, String> {
        if definitions.len() > 16 {
            return Err("Hook owner limit exceeded".into());
        }
        let mut keys = BTreeSet::new();
        let mut replacements = BTreeSet::new();
        let mut owners = Vec::new();
        for (index, definition) in definitions.iter().enumerate() {
            if definition.key.is_empty()
                || definition.key.len() > 512
                || !keys.insert(definition.key.clone())
            {
                return Err("Invalid or duplicate hook owner".into());
            }
            let mut dependency_set = BTreeSet::new();
            for &dependency in &definition.dependencies {
                if dependency >= definitions.len()
                    || dependency == index
                    || !dependency_set.insert(dependency)
                {
                    return Err("Invalid hook dependency".into());
                }
            }
            let program = match &definition.module {
                ModuleCode::Script(bundle) => {
                    bundle.validate()?;
                    if let Some(module) = &bundle.hooks {
                        for binding in &module.bindings {
                            if !definition
                                .grants
                                .iter()
                                .any(|grant| grant == binding.point.capability())
                            {
                                return Err(format!(
                                    "{}: hook {} lacks {}",
                                    definition.key,
                                    binding.point.name(),
                                    binding.point.capability()
                                ));
                            }
                            if binding.mode == HookMode::Replace
                                && !replacements.insert(binding.point)
                            {
                                return Err(format!(
                                    "Ambiguous exclusive replacements at {}",
                                    binding.point.name()
                                ));
                            }
                        }
                        let mut engine = engine();
                        let ast = compile(bundle, module, &engine)?;
                        let operations = Arc::new(AtomicU64::new(0));
                        let operation_limit = Arc::new(AtomicU64::new(MAX_HOOK_OPERATIONS));
                        let counter = operations.clone();
                        let limit = operation_limit.clone();
                        engine.on_progress(move |_| {
                            (counter.fetch_add(1, Ordering::Relaxed)
                                >= limit.load(Ordering::Relaxed))
                            .then(|| Dynamic::from("Hook operation budget exceeded"))
                        });
                        Some(Program {
                            engine,
                            ast,
                            operations,
                            operation_limit,
                            bindings: module.bindings.clone(),
                        })
                    } else {
                        None
                    }
                }
                ModuleCode::Wasm(_) => None,
            };
            owners.push(Owner {
                key: definition.key.clone(),
                dependencies: definition.dependencies.clone(),
                disabled: false,
                program,
            });
        }
        let dispatcher = Self {
            owners,
            snapshots: BTreeMap::new(),
            pending_snapshots: BTreeMap::new(),
            frame: None,
            calls: 0,
            operations: 0,
            diagnostics: Vec::new(),
        };
        dispatcher.ordered_bindings(HookPoint::CameraTransform, HookMode::Before)?;
        Ok(dispatcher)
    }
    pub fn begin_frame(&mut self, frame_id: u64) -> bool {
        if self.frame.is_some_and(|frame| frame_id <= frame) {
            return false;
        }
        self.frame = Some(frame_id);
        self.calls = 0;
        self.operations = 0;
        self.snapshots.append(&mut self.pending_snapshots);
        true
    }
    pub fn set_snapshot(&mut self, owner: &str, snapshot: Value) -> Result<(), String> {
        if !self
            .owners
            .iter()
            .any(|entry| entry.key == owner && !entry.disabled)
        {
            return Err("Unknown or disabled hook snapshot owner".into());
        }
        bounded_json(&snapshot)?;
        self.pending_snapshots.insert(owner.into(), snapshot);
        Ok(())
    }
    pub fn accepts_snapshot(&self, key: &str) -> bool {
        self.owners
            .iter()
            .any(|owner| owner.key == key && !owner.disabled)
    }
    pub fn has(&self, point: HookPoint) -> bool {
        self.owners.iter().any(|owner| {
            !owner.disabled
                && owner
                    .program
                    .as_ref()
                    .is_some_and(|p| p.bindings.iter().any(|b| b.point == point))
        })
    }
    pub fn invoke(&mut self, point: HookPoint, context: Value) -> HookOutcome {
        self.invoke_with(point, context, |value| value)
    }
    pub fn invoke_with<F>(&mut self, point: HookPoint, input: Value, original: F) -> HookOutcome
    where
        F: Fn(Value) -> Value,
    {
        let started = self.operations;
        let mut context = input.clone();
        let mut cancelled = false;
        let mut replaced = false;
        let mut error = validate_context(point, &input)
            .err()
            .map(|error| HookDiagnostic {
                owner: String::new(),
                point,
                function: String::new(),
                error,
            });
        if error.is_none() {
            for mode in [HookMode::Before, HookMode::Cancel, HookMode::Replace] {
                if cancelled {
                    break;
                }
                if let Err(e) = self.stage(point, mode, &mut context, &mut cancelled, &mut replaced)
                {
                    error = Some(e);
                    break;
                }
            }
        }
        if error.is_none() && !cancelled && !replaced {
            context = original(context);
            if let Err(error_message) = validate_context(point, &context) {
                error = Some(HookDiagnostic {
                    owner: String::new(),
                    point,
                    function: String::new(),
                    error: format!("Invalid host hook result: {error_message}"),
                });
            }
        }
        if error.is_none() {
            for mode in [HookMode::Modify, HookMode::After] {
                if mode == HookMode::Modify && cancelled {
                    continue;
                }
                if let Err(e) = self.stage(point, mode, &mut context, &mut cancelled, &mut replaced)
                {
                    error = Some(e);
                    break;
                }
            }
        }
        let mut diagnostics = Vec::new();
        if let Some(diagnostic) = error {
            if !diagnostic.owner.is_empty() {
                self.disable_owner(&diagnostic.owner);
            }
            if self.diagnostics.len() == MAX_DIAGNOSTICS {
                self.diagnostics.remove(0);
            }
            self.diagnostics.push(diagnostic.clone());
            diagnostics.push(diagnostic);
            context = original(input);
            cancelled = false;
            if let Err(error) = validate_context(point, &context) {
                diagnostics.push(HookDiagnostic {
                    owner: String::new(),
                    point,
                    function: String::new(),
                    error: format!("Invalid fallback: {error}"),
                });
            }
        }
        HookOutcome {
            context,
            cancelled,
            diagnostics,
            operations: self.operations.saturating_sub(started),
        }
    }
    fn stage(
        &mut self,
        point: HookPoint,
        mode: HookMode,
        context: &mut Value,
        cancelled: &mut bool,
        replaced: &mut bool,
    ) -> Result<(), HookDiagnostic> {
        let bindings = self
            .ordered_bindings(point, mode)
            .map_err(|error| HookDiagnostic {
                owner: String::new(),
                point,
                function: String::new(),
                error,
            })?;
        for (owner_index, binding) in bindings {
            if self.owners[owner_index].disabled {
                continue;
            }
            if point == HookPoint::ItemTooltipLayout
                && context["owner"].as_str() != Some(self.owners[owner_index].key.as_str())
            {
                continue;
            }
            let owner_key = self.owners[owner_index].key.clone();
            let input = context.clone();
            let snapshot = self
                .snapshots
                .get(&owner_key)
                .cloned()
                .unwrap_or_else(|| json!({}));
            let envelope = json!(
                { "context" : input, "state" : snapshot, "cancelled":* cancelled }
            );
            let fail = |error: String| HookDiagnostic {
                owner: owner_key.clone(),
                point,
                function: binding.function.clone(),
                error: error.chars().take(512).collect(),
            };
            if self.calls >= MAX_FRAME_CALLS || self.operations >= MAX_FRAME_OPERATIONS {
                let mut diagnostic = fail("Aggregate hook frame budget exceeded".into());
                diagnostic.owner.clear();
                return Err(diagnostic);
            }
            self.calls += 1;
            let program = self.owners[owner_index].program.as_mut().unwrap();
            let tooltip = point == HookPoint::ItemTooltip;
            program.engine.set_max_array_size(if tooltip {
                el_mod_content::tooltip::MAX_NODES
            } else {
                512
            });
            program.engine.set_max_map_size(if tooltip {
                el_mod_content::tooltip::MAX_NODES
            } else {
                128
            });
            program.engine.set_max_string_size(if tooltip {
                el_mod_content::tooltip::MAX_BYTES
            } else {
                256
            });
            program.operations.store(0, Ordering::Relaxed);
            let remaining = MAX_FRAME_OPERATIONS - self.operations;
            let per_hook = if tooltip {
                MAX_TOOLTIP_HOOK_OPERATIONS
            } else {
                MAX_HOOK_OPERATIONS
            };
            program
                .operation_limit
                .store(per_hook.min(remaining), Ordering::Relaxed);
            program.engine.set_max_operations(per_hook.min(remaining));
            let argument = rhai::serde::to_dynamic(envelope).map_err(|e| fail(e.to_string()))?;
            let result = program.engine.call_fn_with_options::<Dynamic>(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program.ast,
                &binding.function,
                (argument,),
            );
            self.operations = self
                .operations
                .saturating_add(program.operations.load(Ordering::Relaxed));
            if result.is_ok()
                && program.operations.load(Ordering::Relaxed) > per_hook.min(remaining)
            {
                let mut diagnostic = fail(
                    if remaining < per_hook {
                        "Aggregate hook frame budget exceeded"
                    } else {
                        "Hook operation budget exceeded"
                    }
                    .into(),
                );
                if remaining < per_hook {
                    diagnostic.owner.clear();
                }
                return Err(diagnostic);
            }
            let result = result.map_err(|error| {
                if remaining < per_hook
                    && (program.operations.load(Ordering::Relaxed) >= remaining
                        || matches!(
                            error.unwrap_inner(),
                            rhai::EvalAltResult::ErrorTooManyOperations(_)
                        ))
                {
                    let mut diagnostic = fail("Aggregate hook frame budget exceeded".into());
                    diagnostic.owner.clear();
                    diagnostic
                } else {
                    fail(error.to_string())
                }
            })?;
            if mode == HookMode::Cancel {
                let value = result
                    .try_cast::<bool>()
                    .ok_or_else(|| fail("Cancel hook must return bool".into()))?;
                *cancelled |= value;
                if *cancelled {
                    break;
                }
                continue;
            }
            if result.is_unit()
                && matches!(mode, HookMode::Before | HookMode::After | HookMode::Modify)
            {
                continue;
            }
            if *cancelled {
                return Err(fail(
                    "After a cancelled operation only neutral observation is allowed".into(),
                ));
            }
            bounded_dynamic(
                &result,
                0,
                &mut 0,
                match point {
                    HookPoint::ItemTooltip => el_mod_content::tooltip::MAX_NODES,
                    _ => MAX_NODES,
                },
            )
            .map_err(&fail)?;
            let mut value: Value =
                rhai::serde::from_dynamic(&result).map_err(|e| fail(e.to_string()))?;
            if point == HookPoint::ItemTooltip {
                if ["version", "stack", "contents", "modifiers", "buttons"]
                    .iter()
                    .any(|field| value[*field] != context[*field])
                {
                    return Err(fail(
                        "Tooltip hooks cannot rewrite stack observations or input".into(),
                    ));
                }
                if !value["canvas"].is_null() && value["canvas"] != context["canvas"] {
                    let canvas = value["canvas"]
                        .as_object_mut()
                        .ok_or_else(|| fail("Tooltip canvas must be an object".into()))?;
                    canvas.insert("owner".into(), Value::String(owner_key.clone()));
                }
            }
            if point == HookPoint::ItemTooltipLayout
                && [
                    "version",
                    "owner",
                    "viewport",
                    "pointer",
                    "tooltip",
                    "component_origin",
                    "canvas_size",
                    "align",
                ]
                .iter()
                .any(|field| value[*field] != context[*field])
            {
                return Err(fail(
                    "Tooltip layout hooks cannot rewrite engine layout facts".into(),
                ));
            }
            validate_context(point, &value).map_err(&fail)?;
            if point == HookPoint::InputAction && value != *context {
                return Err(fail(
                    "Input hooks may observe/cancel but cannot rewrite physical actions".into(),
                ));
            }
            *context = value;
            if mode == HookMode::Replace {
                *replaced = true;
            }
        }
        Ok(())
    }
    fn ordered_bindings(
        &self,
        point: HookPoint,
        mode: HookMode,
    ) -> Result<Vec<(usize, HookBinding)>, String> {
        let mut remaining: BTreeSet<usize> = (0..self.owners.len()).collect();
        let mut done = BTreeSet::new();
        let mut bindings = Vec::new();
        let mut pending: Vec<Vec<HookBinding>> = self
            .owners
            .iter()
            .map(|owner| {
                let mut own: Vec<_> = owner
                    .program
                    .as_ref()
                    .into_iter()
                    .flat_map(|program| &program.bindings)
                    .filter(|binding| binding.point == point && binding.mode == mode)
                    .cloned()
                    .collect();
                own.sort_by(|a, b| {
                    (a.priority, a.function.as_str()).cmp(&(b.priority, b.function.as_str()))
                });
                own
            })
            .collect();
        while !remaining.is_empty() {
            let empty_ready: Vec<_> = remaining
                .iter()
                .copied()
                .filter(|&i| {
                    pending[i].is_empty()
                        && self.owners[i].dependencies.iter().all(|d| done.contains(d))
                })
                .collect();
            if !empty_ready.is_empty() {
                for owner in empty_ready {
                    remaining.remove(&owner);
                    done.insert(owner);
                }
                continue;
            }
            let next = remaining
                .iter()
                .copied()
                .filter(|&i| self.owners[i].dependencies.iter().all(|d| done.contains(d)))
                .min_by_key(|&i| {
                    let binding = &pending[i][0];
                    (
                        binding.priority,
                        self.owners[i].key.as_str(),
                        binding.function.as_str(),
                    )
                })
                .ok_or("Cyclic hook dependencies")?;
            bindings.push((next, pending[next].remove(0)));
            if pending[next].is_empty() {
                remaining.remove(&next);
                done.insert(next);
            }
        }
        Ok(bindings)
    }
    pub fn disable_owner(&mut self, key: &str) {
        let Some(index) = self.owners.iter().position(|owner| owner.key == key) else {
            return;
        };
        self.owners[index].disabled = true;
        loop {
            let newly: Vec<_> = self
                .owners
                .iter()
                .enumerate()
                .filter(|(_, owner)| {
                    !owner.disabled && owner.dependencies.iter().any(|&d| self.owners[d].disabled)
                })
                .map(|(i, _)| i)
                .collect();
            if newly.is_empty() {
                break;
            }
            for index in newly {
                self.owners[index].disabled = true;
            }
        }
        for owner in self.owners.iter().filter(|owner| owner.disabled) {
            self.snapshots.remove(&owner.key);
            self.pending_snapshots.remove(&owner.key);
        }
    }
    pub fn reset_world(&mut self) {
        self.snapshots.clear();
        self.pending_snapshots.clear();
    }
    pub fn clear(&mut self) {
        self.owners.clear();
        self.snapshots.clear();
        self.pending_snapshots.clear();
    }
    pub fn report(&self) -> Value {
        json!(
            { "version" : 1, "frame" : self.frame, "calls" : self.calls, "operations" :
            self.operations, "limits" : { "calls" : MAX_FRAME_CALLS, "operations" :
            MAX_FRAME_OPERATIONS, "per_hook_operations" : MAX_HOOK_OPERATIONS,
            "tooltip_hook_operations" : MAX_TOOLTIP_HOOK_OPERATIONS }, "owners" : self
            .owners.iter().map(| o | json!({ "key" : o.key, "disabled" : o.disabled,
            "bindings" : o.program.as_ref().map(| p | p.bindings.len()).unwrap_or(0) }))
            .collect::< Vec < _ >> (), "diagnostics" : self.diagnostics }
        )
    }
}
fn bounded_dynamic(
    value: &Dynamic,
    depth: usize,
    nodes: &mut usize,
    max_nodes: usize,
) -> Result<(), String> {
    *nodes += 1;
    if depth > 8 || *nodes > max_nodes {
        return Err("Hook result depth/node limit exceeded".into());
    }
    if value.is::<Map>() {
        let map = value.read_lock::<Map>().ok_or("Invalid hook map")?;
        for (key, value) in map.iter() {
            if key.len() > 64 {
                return Err("Hook map key too long".into());
            }
            bounded_dynamic(value, depth + 1, nodes, max_nodes)?;
        }
    } else if value.is::<Array>() {
        let array = value.read_lock::<Array>().ok_or("Invalid hook array")?;
        for value in array.iter() {
            bounded_dynamic(value, depth + 1, nodes, max_nodes)?;
        }
    } else if value.is::<rhai::FLOAT>() {
        if !value.as_float().map_err(|e| e.to_string())?.is_finite() {
            return Err("Nonfinite hook result".into());
        }
    } else if !(value.is_unit()
        || value.is::<bool>()
        || value.is::<rhai::INT>()
        || value.is::<rhai::ImmutableString>())
    {
        return Err("Hook result must contain ordinary JSON values".into());
    }
    Ok(())
}
fn bounded_json(value: &Value) -> Result<(), String> {
    bounded_json_limits(value, MAX_DATA_BYTES, MAX_NODES)
}
fn bounded_json_limits(value: &Value, max_bytes: usize, max_nodes: usize) -> Result<(), String> {
    bounded_json_limits_with_strings(value, max_bytes, max_nodes, 256)
}
fn bounded_json_limits_with_strings(
    value: &Value,
    max_bytes: usize,
    max_nodes: usize,
    max_string: usize,
) -> Result<(), String> {
    fn visit(
        value: &Value,
        depth: usize,
        nodes: &mut usize,
        max_nodes: usize,
        max_string: usize,
    ) -> Result<(), String> {
        *nodes += 1;
        if depth > 8 || *nodes > max_nodes {
            return Err("Hook JSON depth/node limit exceeded".into());
        }
        match value {
            Value::Array(values) => {
                for value in values {
                    visit(value, depth + 1, nodes, max_nodes, max_string)?;
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    if key.len() > 64 {
                        return Err("Hook JSON key too long".into());
                    }
                    visit(value, depth + 1, nodes, max_nodes, max_string)?;
                }
            }
            Value::String(value) if value.len() > max_string => {
                return Err("Hook JSON string too long".into());
            }
            Value::Number(value)
                if !value.as_f64().is_some_and(|n| {
                    n.is_finite()
                        && (!(value.is_i64() || value.is_u64())
                            || n.abs() <= 9_007_199_254_740_991.0)
                }) =>
            {
                return Err("Hook JSON number is outside portable finite range".into());
            }
            _ => {}
        }
        Ok(())
    }
    visit(value, 0, &mut 0, max_nodes, max_string)?;
    if serde_json::to_vec(value).map_err(|e| e.to_string())?.len() > max_bytes {
        return Err("Hook JSON byte limit exceeded".into());
    }
    Ok(())
}
pub fn validate_context(point: HookPoint, value: &Value) -> Result<(), String> {
    if point == HookPoint::ItemTooltipLayout {
        bounded_json(value)?;
        let context: el_mod_content::tooltip::LayoutContext = serde_json::from_value(value.clone())
            .map_err(|e| format!("Invalid tooltip layout context: {e}"))?;
        return context.validate();
    }
    if point == HookPoint::ItemTooltip {
        bounded_json_limits_with_strings(
            value,
            el_mod_content::tooltip::MAX_BYTES,
            el_mod_content::tooltip::MAX_NODES,
            2048,
        )?;
        let context: el_mod_content::tooltip::Context = serde_json::from_value(value.clone())
            .map_err(|e| format!("Invalid tooltip context: {e}"))?;
        context.validate()?;
        if let Some(canvas) = context.canvas {
            for widget in canvas.widgets {
                if let el_mod_content::tooltip::Widget::Image { texture, .. } = widget {
                    let owner: el_extension_api::ExtensionId = canvas
                        .owner
                        .rsplit_once('/')
                        .ok_or("Invalid tooltip owner")?
                        .0
                        .parse()
                        .map_err(|_| "Invalid tooltip owner")?;
                    let id: el_mod_content::ContentId =
                        texture.parse().map_err(|_| "Invalid tooltip texture")?;
                    if id.namespace() != owner.namespace() {
                        return Err("Foreign tooltip image namespace".into());
                    }
                }
            }
        }
        return Ok(());
    }
    bounded_json(value)?;
    let object = value.as_object().ok_or("Hook context must be an object")?;
    if object.get("version").and_then(Value::as_u64) != Some(1) {
        return Err("Unsupported hook context version".into());
    }
    let fields: &[&str] = match point {
        HookPoint::CameraTransform => &[
            "version",
            "hurt_scale_milli",
            "yaw_offset_radians",
            "pitch_offset_radians",
            "roll_offset_radians",
            "fov_scale_milli",
        ],
        HookPoint::CapeGeometry => &["version", "offsets_pixel_milli"],
        HookPoint::CameraHurt => &[
            "version",
            "enabled",
            "death_roll_degrees",
            "hurt_roll_degrees",
            "hurt_yaw_degrees",
        ],
        HookPoint::HudHealthBlink => &["version", "blinking"],
        HookPoint::InputAction => &["version", "action", "pressed"],
        _ => &["version", "visible"],
    };
    if object.len() != fields.len() || fields.iter().any(|key| !object.contains_key(*key)) {
        return Err("Hook context has missing or unsupported fields".into());
    }
    match point {
        HookPoint::CameraHurt => {
            if !value["enabled"].is_boolean()
                || [
                    "death_roll_degrees",
                    "hurt_roll_degrees",
                    "hurt_yaw_degrees",
                ]
                .iter()
                .any(|key| {
                    !value[*key]
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && n.abs() <= f64::from(f32::MAX))
                })
            {
                return Err("Invalid hurt-camera render angles".into());
            }
        }
        HookPoint::HudHealthBlink => {
            if !value["blinking"].is_boolean() {
                return Err("Health blinking requires a boolean".into());
            }
        }
        HookPoint::CameraTransform => {
            if !value["hurt_scale_milli"]
                .as_u64()
                .is_some_and(|n| n <= 1000)
                || !value["fov_scale_milli"]
                    .as_u64()
                    .is_some_and(|n| (250..=4000).contains(&n))
            {
                return Err("Camera integer contribution out of bounds".into());
            }
            for name in [
                "yaw_offset_radians",
                "pitch_offset_radians",
                "roll_offset_radians",
            ] {
                if !value[name]
                    .as_f64()
                    .is_some_and(|n| n.is_finite() && n.abs() <= std::f64::consts::TAU)
                {
                    return Err("Camera angular contribution out of bounds".into());
                }
            }
        }
        HookPoint::CapeGeometry => {
            let rows = value["offsets_pixel_milli"]
                .as_array()
                .ok_or("Cape rows must be an array")?;
            if rows.len() != 17 {
                return Err("Cape geometry requires seventeen rows".into());
            }
            for (index, row) in rows.iter().enumerate() {
                let row = row.as_array().ok_or("Cape row must be an array")?;
                if row.len() != 3
                    || row.iter().any(|n| {
                        !n.as_i64()
                            .is_some_and(|n| (-8000..=8000).contains(&n) && (index != 0 || n == 0))
                    })
                {
                    return Err("Cape row dimensions/offset/pinned root out of bounds".into());
                }
            }
        }
        HookPoint::InputAction => {
            let action = value["action"]
                .as_str()
                .ok_or("Input action must be a name")?;
            if action.is_empty()
                || action.len() > 64
                || !action
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                || !value["pressed"].is_boolean()
            {
                return Err("Invalid physical input action".into());
            }
        }
        _ => {
            if !value["visible"].is_boolean() {
                return Err("Visibility result must be boolean".into());
            }
        }
    }
    Ok(())
}
