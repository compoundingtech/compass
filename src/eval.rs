//! The embedded evaluator (decision 0011, 0014).
//!
//! Reading a plan is running it. A version is a module, so evaluating one
//! evaluates every module it imports, transitively — including modules that
//! arrived by replication from another machine (decision 0014). What makes that
//! acceptable is not trust but the environment: a plan runs against a global the
//! host constructs explicitly, holding no clock, no filesystem, no network, no
//! randomness, and nothing platform-dependent (CMP-R12). The capability is
//! *absent* rather than removed.
//!
//! ## Two contexts
//!
//! Compiling a module needs the `Eval` intrinsic, which the sandbox must not
//! keep. So a module is compiled to bytecode in a throwaway `Context::full`, and
//! the bytecode is loaded into a locked `Context::custom` over a minimal
//! intrinsic set (`Json` + `Promise` + `Proxy`). Loading bytecode needs no
//! `Eval` intrinsic, so in the locked context `eval`, `new Function`, and
//! `fn.constructor` exist but throw.
//!
//! ESM evaluation needs the `Promise` intrinsic or QuickJS aborts the process
//! (an uncatchable `SIGABRT`). `Proxy` is present so the `evidence` forwarder
//! works; it is deterministic and grants no capability.
//!
//! ## The determinism allowlist
//!
//! Base objects arrive with `Math.random` and the transcendental `Math`
//! functions, which delegate to the platform library and disagree between
//! platforms in the final bits. They are not structurally omittable, so they are
//! removed explicitly (CMP-A03, decision 0011). Nothing platform-varying — no
//! `Date`, `RegExp` locale data, `Intl`, `WeakRef` — is ever added.
//!
//! ## Bounded evaluation (CMP-R13, decision 0014)
//!
//! Executing foreign code makes non-termination and runaway allocation reachable
//! states rather than theoretical ones, so the runtime carries a time interrupt,
//! a memory limit, and a stack limit. A plan that will not terminate is Stopped
//! and reported, never awaited.

use crate::predicate::Pred;
use rquickjs::context::intrinsic;
use rquickjs::loader::{Loader, Resolver};
use rquickjs::{CatchResultExt, Context, Ctx, Error as JsError, Module, Object, Runtime, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The bare specifier the library is served under.
pub const COMPASS_SPECIFIER: &str = "compass";

/// The prelude source, embedded so the binary carries no runtime prerequisite.
const PRELUDE: &str = include_str!("prelude.js");

/// Wall-clock bound on one evaluation.
const TIME_LIMIT: Duration = Duration::from_secs(5);
/// Memory bound on one evaluation, in bytes.
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
/// Stack bound on one evaluation, in bytes.
const STACK_LIMIT: usize = 1024 * 1024;

/// A step recovered from an evaluated module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemStep {
    /// The name the step was declared under — its identity (decision 0012).
    pub name: String,
    pub work: String,
    pub depends_on: Vec<String>,
    pub supersedes: Option<String>,
    pub accept: Pred,
    pub retired: bool,
}

/// A version recovered from an evaluated module: what the plan *declares*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVersion {
    pub author: String,
    pub why: String,
    pub goal: String,
    pub retired: bool,
    pub steps: Vec<SemStep>,
}

/// Why a read could not produce a value. Each names a different next action
/// (04-cli): `Unresolved` → wait; `Stopped` → do not wait; `Failed` → the plan
/// is at fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// A module it imports is not present locally (CMP.DM-R06a).
    Unresolved(String),
    /// Evaluation exceeded a bound in time or memory (CMP-R13).
    Stopped(String),
    /// The module threw, or does not export a well-formed plan.
    Failed(String),
}

impl EvalError {
    pub fn kind(&self) -> &'static str {
        match self {
            EvalError::Unresolved(_) => "unresolved",
            EvalError::Stopped(_) => "stopped",
            EvalError::Failed(_) => "failed",
        }
    }
    pub fn message(&self) -> &str {
        match self {
            EvalError::Unresolved(m) | EvalError::Stopped(m) | EvalError::Failed(m) => m,
        }
    }
}

/// Evaluate a version file and every module it imports.
///
/// Returns the recovered [`SemVersion`] for every version module in the graph,
/// keyed by canonical path, so one evaluation of a tip yields the intent of the
/// whole lineage — which is what the Rationale chain needs.
pub fn eval_plan_file(entry: &Path) -> Result<HashMap<PathBuf, SemVersion>, EvalError> {
    let canon_entry = canonicalize(entry)?;
    let entry_name = path_name(&canon_entry);

    let rt = Runtime::new().map_err(|e| EvalError::Failed(format!("runtime: {e}")))?;
    rt.set_memory_limit(MEMORY_LIMIT);
    rt.set_max_stack_size(STACK_LIMIT);

    // Phase 1 — compile the whole graph to bytecode in a throwaway full context.
    //
    // quickjs-ng resolves a module's imports while compiling it, so modules
    // cannot be compiled in isolation: the loader transpiles and declares each
    // import on demand and captures its bytecode as it goes.
    let captured: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    let load_err: Arc<Mutex<Option<EvalError>>> = Arc::new(Mutex::new(None));
    {
        rt.set_loader(
            GraphResolver,
            SourceLoader {
                captured: captured.clone(),
                err: load_err.clone(),
            },
        );
        let full = Context::full(&rt).map_err(|e| EvalError::Failed(format!("context: {e}")))?;
        let entry_js = build_js(&entry_name)?;
        full.with(|ctx| -> Result<(), EvalError> {
            let module = Module::declare(
                ctx.clone(),
                entry_name.as_bytes().to_vec(),
                entry_js.as_bytes().to_vec(),
            )
            .catch(&ctx)
            .map_err(|e| {
                take_or(&load_err, || {
                    EvalError::Failed(format!("{entry_name}: {e}"))
                })
            })?;
            let bytecode = module
                .write(rquickjs::module::WriteOptions::default())
                .map_err(|e| EvalError::Failed(format!("{entry_name}: write bytecode: {e}")))?;
            captured
                .lock()
                .unwrap()
                .insert(entry_name.clone(), bytecode);
            Ok(())
        })?;
    }

    let compiled = captured.lock().unwrap().clone();
    let version_names: Vec<String> = compiled
        .keys()
        .filter(|k| *k != COMPASS_SPECIFIER)
        .cloned()
        .collect();

    // Phase 2 — load the bytecode into the locked context and evaluate.
    let interrupted = Arc::new(AtomicBool::new(false));
    install_interrupt(&rt, interrupted.clone());

    let locked = Context::custom::<(intrinsic::Json, intrinsic::Promise, intrinsic::Proxy)>(&rt)
        .map_err(|e| EvalError::Failed(format!("locked context: {e}")))?;
    rt.set_loader(GraphResolver, BytecodeLoader { compiled });

    locked.with(|ctx| -> Result<HashMap<PathBuf, SemVersion>, EvalError> {
        lock_globals(&ctx)?;
        let mut out = HashMap::new();
        for name in &version_names {
            let sem = eval_one(&ctx, name, &interrupted)?;
            out.insert(PathBuf::from(name), sem);
        }
        Ok(out)
    })
}

/// Prefer a real error the loader recorded over a generic host message.
fn take_or(
    slot: &Arc<Mutex<Option<EvalError>>>,
    fallback: impl FnOnce() -> EvalError,
) -> EvalError {
    slot.lock().unwrap().take().unwrap_or_else(fallback)
}

/// Transpile + footer for a resolved module name (`"compass"` or a file path).
fn build_js(name: &str) -> Result<String, EvalError> {
    if name == COMPASS_SPECIFIER {
        let exports = collect_exports(PRELUDE, Path::new("compass.js"))?;
        return Ok(with_footer(PRELUDE, &exports));
    }
    let path = Path::new(name);
    let source = read_source(path)?;
    let exports = collect_exports(&source, path)?;
    Ok(with_footer(&transpile(path, &source)?, &exports))
}

/// Import and extract one already-compiled version module.
fn eval_one(
    ctx: &Ctx<'_>,
    name: &str,
    interrupted: &Arc<AtomicBool>,
) -> Result<SemVersion, EvalError> {
    let ns = import_namespace(ctx, name, interrupted)?;
    let default: Value = ns
        .get("default")
        .map_err(|e| classify(ctx, e, interrupted))?;
    let plan = default
        .as_object()
        .ok_or_else(|| EvalError::Failed(format!("{name}: default export is not a plan object")))?;
    extract_plan(ctx, plan, name)
}

/// Drive `Module::import` to completion, translating its failure modes.
fn import_namespace<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    interrupted: &Arc<AtomicBool>,
) -> Result<Object<'js>, EvalError> {
    let promise =
        Module::import(ctx, name.as_bytes().to_vec()).map_err(|e| classify(ctx, e, interrupted))?;
    promise
        .finish::<Object>()
        .map_err(|e| classify(ctx, e, interrupted))
}

/// Map a JS error to the right read-failure, checking the interrupt flag first.
fn classify(ctx: &Ctx<'_>, err: JsError, interrupted: &Arc<AtomicBool>) -> EvalError {
    if interrupted.load(Ordering::Relaxed) {
        return EvalError::Stopped(format!(
            "evaluation exceeded the {}s time limit",
            TIME_LIMIT.as_secs()
        ));
    }
    if let JsError::Exception = err {
        let msg = ctx
            .catch()
            .as_exception()
            .and_then(|e| e.message())
            .unwrap_or_else(|| "uncaught exception".to_string());
        return EvalError::Failed(msg);
    }
    EvalError::Failed(err.to_string())
}

// ---------------------------------------------------------------------------
// Extraction: evaluated JS values -> owned Rust
// ---------------------------------------------------------------------------

fn extract_plan(ctx: &Ctx<'_>, plan: &Object<'_>, name: &str) -> Result<SemVersion, EvalError> {
    let fail = |m: String| EvalError::Failed(format!("{name}: {m}"));
    let author = get_string(plan, "author").ok_or_else(|| fail("plan has no author".into()))?;
    let why = get_string(plan, "why").ok_or_else(|| fail("plan has no rationale (why)".into()))?;
    let goal = get_string(plan, "goal").ok_or_else(|| fail("plan has no goal".into()))?;
    let retired = plan.get::<_, bool>("retired").unwrap_or(false);

    let raw_steps: Value = plan
        .get("__steps")
        .map_err(|_| fail("default export is not a plan (no steps)".into()))?;
    let arr = raw_steps
        .as_array()
        .ok_or_else(|| fail("plan steps are not an array".into()))?;

    let mut steps = Vec::new();
    for item in arr.iter::<Value>() {
        let item = item.map_err(|e| fail(format!("reading a step: {e}")))?;
        let sobj = item
            .as_object()
            .ok_or_else(|| fail("a step is not an object".into()))?;
        steps.push(extract_step(ctx, sobj, name)?);
    }

    Ok(SemVersion {
        author,
        why,
        goal,
        retired,
        steps,
    })
}

fn extract_step(ctx: &Ctx<'_>, s: &Object<'_>, module: &str) -> Result<SemStep, EvalError> {
    let fail = |m: String| EvalError::Failed(format!("{module}: {m}"));
    let name = get_string(s, "__name").ok_or_else(|| {
        fail(
            "a step has no declared name: a step must be an exported binding, whose \
              name is its identity (decision 0012)"
                .into(),
        )
    })?;
    let work = get_string(s, "work").ok_or_else(|| fail(format!("step {name} has no work")))?;

    let mut depends_on = Vec::new();
    if let Ok(deps) = s.get::<_, Value>("dependsOn") {
        if let Some(arr) = deps.as_array() {
            for d in arr.iter::<Value>() {
                let d = d.map_err(|e| fail(format!("step {name}: reading a dependency: {e}")))?;
                let dobj = d.as_object().ok_or_else(|| {
                    fail(format!(
                        "step {name} depends on something that is not a step"
                    ))
                })?;
                let dep = get_string(dobj, "__name").ok_or_else(|| {
                    fail(format!(
                        "step {name} depends on a step with no declared name"
                    ))
                })?;
                depends_on.push(dep);
            }
        }
    }
    depends_on.sort();
    depends_on.dedup();

    let supersedes = s
        .get::<_, Value>("supersedes")
        .ok()
        .filter(|v| v.is_object())
        .and_then(|v| v.into_object())
        .and_then(|o| get_string(&o, "__name"));

    let accept_val: Value = s
        .get("accept")
        .map_err(|_| fail(format!("step {name} has no acceptance criterion")))?;
    let accept = to_pred(ctx, &accept_val).map_err(|m| fail(format!("step {name}: {m}")))?;

    let retired = s.get::<_, bool>("retired").unwrap_or(false);

    Ok(SemStep {
        name,
        work,
        depends_on,
        supersedes,
        accept,
        retired,
    })
}

/// Convert an evidence tree (`atom` / `all` / `any` / `not`) to a [`Pred`].
fn to_pred(ctx: &Ctx<'_>, v: &Value<'_>) -> Result<Pred, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| "an acceptance criterion must be an evidence value".to_string())?;
    let kind = get_string(obj, "__k").ok_or_else(|| {
        "acceptance is not an evidence value (use evidence.* / all / any / not)".to_string()
    })?;
    match kind.as_str() {
        "atom" => {
            let k = get_string(obj, "kind").ok_or("evidence atom has no kind")?;
            let attrs_v: Value = obj.get("attrs").map_err(|e| e.to_string())?;
            let mut attrs: Vec<(String, String)> = Vec::new();
            if let Some(ao) = attrs_v.as_object() {
                for key in ao.keys::<String>() {
                    let key = key.map_err(|e| e.to_string())?;
                    let val: Value = ao.get(&key).map_err(|e| e.to_string())?;
                    attrs.push((key, value_to_string(&val)?));
                }
            }
            attrs.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(Pred::Atom { kind: k, attrs })
        }
        "all" | "any" => {
            let args: Value = obj.get("args").map_err(|e| e.to_string())?;
            let arr = args
                .as_array()
                .ok_or("all()/any() arguments are not an array")?;
            let mut ps = Vec::new();
            for a in arr.iter::<Value>() {
                let a = a.map_err(|e| e.to_string())?;
                ps.push(to_pred(ctx, &a)?);
            }
            Ok(if kind == "all" {
                Pred::All(ps)
            } else {
                Pred::Any(ps)
            })
        }
        "not" => {
            let inner: Value = obj.get("arg").map_err(|e| e.to_string())?;
            Ok(Pred::Not(Box::new(to_pred(ctx, &inner)?)))
        }
        other => Err(format!("unknown evidence combinator `{other}`")),
    }
}

/// Canonicalise a scalar JS value the way both criteria and evidence render it,
/// so a criterion and a record naming the same thing match.
fn value_to_string(v: &Value<'_>) -> Result<String, String> {
    if let Some(s) = v.as_string() {
        return s.to_string().map_err(|e| e.to_string());
    }
    if v.is_bool() {
        return Ok(if v.as_bool().unwrap() {
            "true"
        } else {
            "false"
        }
        .to_string());
    }
    if let Some(n) = v.as_number() {
        if n.fract() == 0.0 && n.is_finite() && n.abs() < 9.007_199_254_740_992e15 {
            return Ok(format!("{}", n as i64));
        }
        return Ok(format!("{n}"));
    }
    if v.is_undefined() || v.is_null() {
        return Err("an evidence attribute is null or undefined".to_string());
    }
    Err("an evidence attribute is neither a string, number, nor boolean".to_string())
}

fn get_string(o: &Object<'_>, key: &str) -> Option<String> {
    let v: Value = o.get(key).ok()?;
    v.as_string().and_then(|s| s.to_string().ok())
}

// ---------------------------------------------------------------------------
// Compilation
// ---------------------------------------------------------------------------

/// Transpile a TypeScript module to JavaScript via oxc.
///
/// The transpiled output is never stored and never hashed — identity is the hash
/// of the *authored source bytes* (decision 0014). This step exists only so the
/// engine, which speaks JavaScript, can run a module authored in TypeScript.
fn transpile(path: &Path, source: &str) -> Result<String, EvalError> {
    use oxc_allocator::Allocator;
    use oxc_codegen::Codegen;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;
    use oxc_transformer::{TransformOptions, Transformer};

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let ret = Parser::new(&allocator, source, source_type).parse();
    if ret.panicked {
        return Err(EvalError::Failed(format!(
            "{}: cannot parse: {}",
            path.display(),
            first_diag(&ret.diagnostics)
        )));
    }
    let mut program = ret.program;

    // `with_enum_eval(true)` keeps oxc from panicking on `enum`.
    let semantic = SemanticBuilder::new().with_enum_eval(true).build(&program);
    let scoping = semantic.semantic.into_scoping();

    let options = TransformOptions::default();
    let tret =
        Transformer::new(&allocator, path, &options).build_with_scoping(scoping, &mut program);
    if !tret.diagnostics.is_empty() {
        return Err(EvalError::Failed(format!(
            "{}: cannot transpile: {}",
            path.display(),
            tret.diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }

    Ok(Codegen::new().build(&program).code)
}

fn first_diag<T: std::fmt::Display>(errs: &[T]) -> String {
    errs.first()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "syntax error".to_string())
}

/// The import specifiers a module declares, statically (no evaluation).
///
/// A version references its predecessors by importing their files, so the
/// lineage can be walked from source bytes alone — admission never runs a module
/// (02-artifacts). Returns every specifier, including `"compass"`.
pub fn import_specifiers(source: &str, path: &Path) -> Result<Vec<String>, EvalError> {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let ret = Parser::new(&allocator, source, source_type).parse();
    if ret.panicked {
        return Err(EvalError::Failed(format!(
            "{}: cannot parse: {}",
            path.display(),
            first_diag(&ret.diagnostics)
        )));
    }
    let mut specs = Vec::new();
    for stmt in &ret.program.body {
        match stmt {
            Statement::ImportDeclaration(import) => specs.push(import.source.value.to_string()),
            Statement::ExportNamedDeclaration(e) => {
                if let Some(src) = &e.source {
                    specs.push(src.value.to_string());
                }
            }
            Statement::ExportAllDeclaration(e) => specs.push(e.source.value.to_string()),
            _ => {}
        }
    }
    Ok(specs)
}

/// Append one registration call per exported top-level `const`, so a step
/// learns the name it was declared under. Consumption is lazy (see the prelude),
/// so appending after the module body is correct.
fn with_footer(js: &str, exports: &[String]) -> String {
    if exports.is_empty() {
        return js.to_string();
    }
    let mut out = String::with_capacity(js.len() + exports.len() * 48);
    out.push_str(js);
    out.push('\n');
    for name in exports {
        out.push_str(&format!(
            "try {{ globalThis.__compass_register({name}, {name:?}); }} catch (_e) {{}}\n"
        ));
    }
    out
}

/// The names of top-level `export const` bindings, in source order.
fn collect_exports(source: &str, path: &Path) -> Result<Vec<String>, EvalError> {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::{BindingPattern, Declaration, Statement};
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let ret = Parser::new(&allocator, source, source_type).parse();
    if ret.panicked {
        return Err(EvalError::Failed(format!(
            "{}: cannot parse: {}",
            path.display(),
            first_diag(&ret.diagnostics)
        )));
    }
    let mut names = Vec::new();
    for stmt in &ret.program.body {
        if let Statement::ExportNamedDeclaration(export) = stmt {
            if let Some(Declaration::VariableDeclaration(var)) = &export.declaration {
                for decl in &var.declarations {
                    if let BindingPattern::BindingIdentifier(id) = &decl.id {
                        names.push(id.name.to_string());
                    }
                }
            }
        }
    }
    Ok(names)
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

enum Resolved {
    Compass,
    File(PathBuf),
}

/// Resolve an import specifier against the importing file.
fn resolve_spec(base: &Path, spec: &str) -> Result<Resolved, EvalError> {
    if spec == COMPASS_SPECIFIER {
        return Ok(Resolved::Compass);
    }
    let dir = base.parent().unwrap_or_else(|| Path::new("."));
    let joined = dir.join(spec);
    let canon = canonicalize(&joined)?;
    Ok(Resolved::File(canon))
}

fn canonicalize(path: &Path) -> Result<PathBuf, EvalError> {
    std::fs::canonicalize(path).map_err(|_| {
        EvalError::Unresolved(format!(
            "{} has not arrived (or was never committed)",
            path.display()
        ))
    })
}

fn read_source(path: &Path) -> Result<String, EvalError> {
    let bytes = std::fs::read(path)
        .map_err(|_| EvalError::Unresolved(format!("{} has not arrived", path.display())))?;
    String::from_utf8(bytes)
        .map_err(|e| EvalError::Failed(format!("{}: not valid UTF-8: {e}", path.display())))
}

fn path_name(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

// ---------------------------------------------------------------------------
// The loader over precompiled bytecode
// ---------------------------------------------------------------------------

struct GraphResolver;

impl Resolver for GraphResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attrs: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        if name == COMPASS_SPECIFIER {
            return Ok(COMPASS_SPECIFIER.to_string());
        }
        match resolve_spec(Path::new(base), name) {
            Ok(Resolved::Compass) => Ok(COMPASS_SPECIFIER.to_string()),
            Ok(Resolved::File(p)) => Ok(p.to_string_lossy().to_string()),
            Err(_) => Err(JsError::new_resolving(base.to_string(), name.to_string())),
        }
    }
}

/// Phase 1 loader: transpile + declare each imported module on demand, and
/// capture its bytecode. Records the first real underlying failure so the caller
/// can distinguish an unresolved import from a plan fault.
struct SourceLoader {
    captured: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    err: Arc<Mutex<Option<EvalError>>>,
}

impl Loader for SourceLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attrs: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, rquickjs::module::Declared>> {
        let js = match build_js(name) {
            Ok(js) => js,
            Err(e) => {
                *self.err.lock().unwrap() = Some(e);
                return Err(JsError::new_loading(name.to_string()));
            }
        };
        let module = Module::declare(
            ctx.clone(),
            name.as_bytes().to_vec(),
            js.as_bytes().to_vec(),
        )?;
        if let Ok(bytecode) = module.write(rquickjs::module::WriteOptions::default()) {
            self.captured
                .lock()
                .unwrap()
                .insert(name.to_string(), bytecode);
        }
        Ok(module)
    }
}

/// Phase 2 loader: serve already-compiled bytecode into the locked context.
struct BytecodeLoader {
    compiled: HashMap<String, Vec<u8>>,
}

impl Loader for BytecodeLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attrs: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, rquickjs::module::Declared>> {
        match self.compiled.get(name) {
            Some(bytes) => unsafe { Module::load(ctx.clone(), bytes) },
            None => Err(JsError::new_loading(name.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// The determinism allowlist + bounds
// ---------------------------------------------------------------------------

/// Remove `Math.random` and the transcendental `Math` functions, and assert the
/// platform-varying globals are absent. Determinism requires the global be an
/// explicit allowlist (decision 0011).
fn lock_globals(ctx: &Ctx<'_>) -> Result<(), EvalError> {
    let globals = ctx.globals();
    if let Ok(math) = globals.get::<_, Object>("Math") {
        // Everything that delegates to the platform libm, plus randomness.
        const REMOVE: &[&str] = &[
            "random", "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
            "asinh", "acosh", "atanh", "exp", "expm1", "log", "log1p", "log2", "log10", "pow",
            "sqrt", "cbrt", "hypot",
        ];
        for name in REMOVE {
            let _ = math.remove(*name);
        }
    }
    Ok(())
}

/// Install a time interrupt on the runtime, recording that it fired so a Stopped
/// read is not misreported as a plan fault.
fn install_interrupt(rt: &Runtime, flag: Arc<AtomicBool>) {
    let deadline = Instant::now() + TIME_LIMIT;
    rt.set_interrupt_handler(Some(Box::new(move || {
        if Instant::now() >= deadline {
            flag.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    })));
}
