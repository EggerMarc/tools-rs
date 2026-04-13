//! Python language adapter for tools-rs FFI.
//!
//! Loads Python scripts from a directory, discovers functions decorated
//! with `@tool`, and returns [`PyToolDef`]s ready for conversion to
//! `RawToolDef` by `tools_core`.

mod decorator;

use std::{
    ffi::CString,
    fs,
    future::Future,
    path::Path,
    pin::Pin,
};

use pyo3::{
    prelude::*,
    types::{PyAnyMethods, PyDict, PyList, PyListMethods, PyModule},
};
use serde_json::Value;

/// Boxed future type alias (avoids pulling in `futures` crate).
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// A tool definition extracted from a Python script.
///
/// This mirrors `tools_core::RawToolDef` but lives in this crate to
/// avoid a cyclic dependency. `tools_core` converts these into
/// `RawToolDef` when the `python` feature is enabled.
pub struct PyToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub meta: Value,
    pub func: Box<dyn Fn(Value) -> BoxFuture<Result<Value, String>> + Send + Sync>,
}

/// Load all `@tool`-decorated functions from `*.py` files in `dir`.
///
/// 1. Discovers `*.py` files in `dir`
/// 2. For each file: injects the `@tool` decorator, executes the script,
///    scans for functions with `__tool__` attribute
/// 3. Returns a `PyToolDef` per discovered tool
pub fn load(dir: &Path) -> Result<Vec<PyToolDef>, String> {
    if !dir.is_dir() {
        return Err(format!(
            "Python adapter: path is not a directory: {}",
            dir.display()
        ));
    }

    let mut defs = Vec::new();

    // Collect *.py files
    let entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("Python adapter: failed to read directory {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "py")
        })
        .collect();

    if entries.is_empty() {
        return Ok(defs);
    }

    Python::with_gil(|py| {
        // Detect and configure venv if present
        setup_venv(py, dir);

        for entry in &entries {
            let file_path = entry.path();
            let source = fs::read_to_string(&file_path).map_err(|e| {
                format!(
                    "Python adapter: failed to read {}: {e}",
                    file_path.display()
                )
            })?;

            let file_defs = load_file(py, &file_path, &source)?;
            defs.extend(file_defs);
        }

        Ok(defs)
    })
}

/// Load tools from a single Python file.
fn load_file(py: Python<'_>, path: &Path, source: &str) -> Result<Vec<PyToolDef>, String> {
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Combine decorator source + a fake tools_rs module + user script.
    // The fake module makes `from tools_rs import tool` work so scripts
    // are valid Python both at runtime (auto-injected) and in IDEs (pip
    // package).
    let full_source = format!(
        "{decorator}\n\
         import types as _types\n\
         _tools_rs_mod = _types.ModuleType('tools_rs')\n\
         _tools_rs_mod.tool = tool\n\
         import sys as _sys\n\
         _sys.modules['tools_rs'] = _tools_rs_mod\n\
         del _types, _tools_rs_mod, _sys\n\
         \n\
         {source}",
        decorator = decorator::DECORATOR_SOURCE,
        source = source,
    );
    let c_source = CString::new(full_source)
        .map_err(|_| "Python source contains null bytes".to_string())?;
    let c_file = CString::new(format!("{file_name}.py"))
        .map_err(|_| "file name contains null bytes".to_string())?;
    let c_module = CString::new(file_name)
        .map_err(|_| "module name contains null bytes".to_string())?;

    // Execute in a fresh module
    let module = PyModule::from_code(py, &c_source, &c_file, &c_module)
        .map_err(|e| format!("Python adapter: failed to execute {}: {e}", path.display()))?;

    // Scan for functions with __tool__ attribute
    let mut defs = Vec::new();
    let dir_list = module
        .dir()
        .map_err(|e| format!("Python adapter: failed to list module attrs: {e}"))?;

    for attr_name in dir_list.iter() {
        let attr_name_str: String = attr_name
            .extract()
            .map_err(|e| format!("Python adapter: failed to extract attr name: {e}"))?;

        // Skip dunder and private names
        if attr_name_str.starts_with('_') {
            continue;
        }

        let obj = match module.getattr(&*attr_name_str) {
            Ok(o) => o,
            Err(_) => continue,
        };

        // Check if it has __tool__ attribute
        let tool_dict = match obj.getattr("__tool__") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let def = extract_tool_def(py, &tool_dict, obj)?;
        defs.push(def);
    }

    Ok(defs)
}

/// Extract a `PyToolDef` from a function's `__tool__` dict.
fn extract_tool_def(
    py: Python<'_>,
    tool_dict: &Bound<'_, PyAny>,
    callable: Bound<'_, PyAny>,
) -> Result<PyToolDef, String> {
    let name: String = tool_dict
        .get_item("name")
        .map_err(|e| format!("missing 'name' in __tool__: {e}"))?
        .extract()
        .map_err(|e| format!("'name' is not a string: {e}"))?;

    let description: String = tool_dict
        .get_item("description")
        .map_err(|e| format!("missing 'description' in __tool__: {e}"))?
        .extract()
        .map_err(|e| format!("'description' is not a string: {e}"))?;

    // Convert parameters dict to serde_json::Value via JSON round-trip
    let params_obj = tool_dict
        .get_item("parameters")
        .map_err(|e| format!("missing 'parameters' in __tool__: {e}"))?;
    let parameters = py_to_json(py, &params_obj)?;

    // Convert meta dict to serde_json::Value
    let meta_obj = tool_dict
        .get_item("meta")
        .map_err(|e| format!("missing 'meta' in __tool__: {e}"))?;
    let meta = py_to_json(py, &meta_obj)?;

    // Build the call closure: captures the PyObject, calls via GIL.
    // PyObject (Py<PyAny>) is Send but not Clone outside GIL — wrap
    // in Arc so the Fn closure can be called multiple times.
    let call_fn = std::sync::Arc::new(callable.unbind());

    let func: Box<dyn Fn(Value) -> BoxFuture<Result<Value, String>> + Send + Sync> =
        Box::new(move |args: Value| {
            let call_fn = call_fn.clone(); // Arc clone, cheap
            Box::pin(async move {
                tokio::task::spawn_blocking(move || {
                    Python::with_gil(|py| {
                        let kwargs = json_to_py(py, &args)
                            .map_err(|e| format!("failed to convert args to Python: {e}"))?;
                        let kwargs_dict = kwargs
                            .downcast_bound::<PyDict>(py)
                            .map_err(|e| format!("args must be an object: {e}"))?;

                        let result = call_fn
                            .call(py, (), Some(kwargs_dict))
                            .map_err(|e| format!("Python tool error: {e}"))?;

                        py_to_json(py, result.bind(py))
                    })
                })
                .await
                .map_err(|e| format!("spawn_blocking failed: {e}"))?
            })
        });

    Ok(PyToolDef {
        name,
        description,
        parameters,
        meta,
        func,
    })
}

// ============================================================================
// PYTHON ↔ JSON CONVERSION
// ============================================================================

/// Convert a Python object to a `serde_json::Value` via `json.dumps`.
fn py_to_json(py: Python<'_>, obj: &Bound<'_, PyAny>) -> Result<Value, String> {
    let json_mod = py.import("json").map_err(|e| format!("failed to import json: {e}"))?;
    let json_str: String = json_mod
        .call_method1("dumps", (obj,))
        .map_err(|e| format!("json.dumps failed: {e}"))?
        .extract()
        .map_err(|e| format!("json.dumps didn't return str: {e}"))?;
    serde_json::from_str(&json_str).map_err(|e| format!("invalid JSON from Python: {e}"))
}

/// Convert a `serde_json::Value` to a Python object via `json.loads`.
fn json_to_py(py: Python<'_>, value: &Value) -> Result<PyObject, String> {
    let json_str = serde_json::to_string(value)
        .map_err(|e| format!("failed to serialize JSON: {e}"))?;
    let json_mod = py.import("json").map_err(|e| format!("failed to import json: {e}"))?;
    let obj = json_mod
        .call_method1("loads", (json_str,))
        .map_err(|e| format!("json.loads failed: {e}"))?;
    Ok(obj.unbind())
}

// ============================================================================
// VENV DETECTION
// ============================================================================

/// If a `.venv` directory exists in or above `dir`, prepend its
/// `site-packages` to `sys.path`.
fn setup_venv(py: Python<'_>, dir: &Path) {
    let mut search = Some(dir);
    while let Some(d) = search {
        let venv = d.join(".venv");
        if venv.is_dir() {
            if let Ok(site_packages) = find_site_packages(&venv) {
                let _ = add_to_sys_path(py, &site_packages);
            }
            return;
        }
        search = d.parent();
    }
}

/// Find `site-packages` inside a venv.
fn find_site_packages(venv: &Path) -> Result<String, String> {
    let lib = venv.join("lib");
    if !lib.is_dir() {
        return Err("no lib/ in venv".into());
    }
    // lib/pythonX.Y/site-packages
    for entry in fs::read_dir(&lib).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let sp = entry.path().join("site-packages");
        if sp.is_dir() {
            return sp
                .to_str()
                .map(|s| s.to_string())
                .ok_or("non-UTF8 path".into());
        }
    }
    Err("site-packages not found in venv".into())
}

/// Prepend a path to `sys.path`.
fn add_to_sys_path(py: Python<'_>, path: &str) -> Result<(), String> {
    let sys = py.import("sys").map_err(|e| format!("failed to import sys: {e}"))?;
    let sys_path = sys
        .getattr("path")
        .map_err(|e| format!("failed to get sys.path: {e}"))?;
    let sys_path = sys_path
        .downcast_into::<PyList>()
        .map_err(|e| format!("sys.path is not a list: {e}"))?;
    sys_path
        .insert(0, path)
        .map_err(|e| format!("failed to insert into sys.path: {e}"))?;
    Ok(())
}
