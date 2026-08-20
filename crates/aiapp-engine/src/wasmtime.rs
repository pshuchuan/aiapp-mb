//! Real WASM executor (requires the `wasmtime` feature).
//!
//! Uses Wasmtime + WASI to instantiate the WASM bytecode inside a `.aiapp` package,
//! and links host capabilities into the app as imports per the WIT contract. This is the
//! shared "player" execution core used by all host types
//! (desktop CLI, host App, standalone App).
//!
//! Note: the WIT-spec (`wit-bindgen`) auto-generation layer is provided by the SDK (see `host-app/rust-core`);
//! this module is a reference implementation of a "pluggable executor" — for non-WASI imports it registers a
//! "host-capability bridge matching the signature arity", so that console-style `.aiapp` apps can actually run
//! in the terminal (printing to stdout) before the SDK generation layer is wired in.
//!
//! Known limitation (a toolchain difference, not a defect of this module): the MoonBit `wasm-gc` target emits
//! `array.new_default` in const expressions of global constants, while wasmtime 24's GC implementation does not
//! yet support this instruction, causing module parsing to fail. Workarounds: ① switch to the `wasm-target wasm`
//! (classic MVP/WASI, no GC constants) build; ② upgrade wasmtime to a version that supports const-expr GC
//! instructions and rebuild this feature. Other modules without that trait (e.g. hand-written / older build
//! artifacts) execute normally.
//!
//! Note: this module is synchronous; the host `Host` trait is async, so it builds its own tokio runtime internally
//! and bridges with `block_on` (in CLI scenarios this runs outside a runtime, so there is no nesting issue).

use std::sync::Arc;

use crate::host::Host;
use wasmtime::{
    Caller, Config, Engine, ExternType, FuncType, Linker, Module, Store, Val, ValType,
};

/// Wasmtime executor. Holds a host capability implementation (`Host`) for the app to import.
pub struct WasmExec {
    host: Arc<dyn Host>,
}

impl WasmExec {
    /// Create the executor.
    pub fn new(host: Arc<dyn Host>) -> Self {
        WasmExec { host }
    }

    /// Run a piece of WASM bytecode: parse → link (WASI + host-capability bridge) → call entry.
    ///
    /// Entry lookup order: `_start` (WASI console entry) → `main` (MoonBit export).
    /// Returns an execution summary (success) or an error message.
    pub fn run(&self, wasm: &[u8], app_id: &str) -> Result<String, String> {
        let mut config = Config::new();
        // MoonBit's wasm-gc target output depends on GC / function-references types, which must be explicitly enabled
        config.wasm_function_references(true);
        config.wasm_gc(true);
        let engine = Engine::new(&config).map_err(|e| format!("failed to initialize engine: {e}"))?;
        let module = Module::from_binary(&engine, wasm).map_err(|e| {
            format!(
                "module parsing failed: {e}\nhint: MoonBit wasm-gc output contains const-expr GC instructions \
                 (array.new_default), which wasmtime 24 does not yet support; rebuild with the classic wasm target, \
                 or upgrade wasmtime and rebuild this feature"
            )
        })?;

        // The host Host trait is async: build our own runtime for the capability bridge/logging calls
        let rt = Arc::new(
            tokio::runtime::Runtime::new().map_err(|e| format!("failed to initialize tokio runtime: {e}"))?,
        );

        let mut linker = Linker::new(&engine);

        // A single Store holding the WASI context (spans linking / instantiation / entry call)
        let mut store = Store::new(
            &engine,
            wasmtime_wasi::WasiCtxBuilder::new()
                .inherit_stdout()
                .inherit_stderr()
                .build_p1(),
        );

        // 1) WASI: provides console output (stdout/stderr), console-style apps work out of the box
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
            .map_err(|e| format!("WASI linking failed: {e}"))?;

        // 2) Host-capability bridge: for non-WASI imports, register a no-op stub matching the signature arity
        //    (logs the call, useful for permissions/auditing)
        for import in module.imports() {
            if import.module().starts_with("wasi") {
                continue;
            }
            let ty = match import.ty() {
                ExternType::Func(ft) => ft.clone(),
                _ => continue,
            };
            let param_count = ty.params().len();
            let ftype = FuncType::new(
                &engine,
                ty.params().collect::<Vec<ValType>>(),
                ty.results().collect::<Vec<ValType>>(),
            );
            let host = Arc::clone(&self.host);
            let rt = Arc::clone(&rt);
            let name = import.name().to_string();
            let name_for_err = name.clone();
            let name_for_reg = name.clone();
            linker
                .func_new(
                    import.module(),
                    &name_for_reg,
                    ftype,
                    move |_caller: Caller<'_, _>, _args: &[Val], results: &mut [Val]| {
                        let _ = rt.block_on(host.log(
                            "warn",
                            &format!(
                                "host-capability bridge stub called: {name} ({param_count} args; production wiring goes through SDK wit-bindgen)"
                            ),
                        ));
                        for r in results.iter_mut() {
                            *r = Val::I32(0);
                        }
                        Ok(())
                    },
                )
                .map_err(|e| format!("failed to register host import {name_for_err}: {e}"))?;
        }

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("instantiation failed: {e}"))?;

        // 3) Execute entry
        for n in ["_start", "main"] {
            if let Ok(entry) = instance.get_typed_func::<(), ()>(&mut store, n) {
                let _ = rt.block_on(
                    self.host
                        .log("info", &format!("executing entry {n} (app_id: {app_id})")),
                );
                entry
                    .call(&mut store, ())
                    .map_err(|e| format!("failed to execute {n}: {e}"))?;
                return Ok(format!("executed entry {n} (app_id: {app_id})"));
            }
        }
        Err("module has no callable entry (_start / main)".into())
    }
}
