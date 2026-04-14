//! Python language adapter.
//!
//! Loads Python scripts from a directory, discovers functions decorated
//! with `@tool`, and returns [`RawToolDef`]s for registration.

mod decorator;

use std::{ffi::CString, fs, path::Path, sync::Arc};

use pyo3::{
    prelude::*,
    types::{PyAnyMethods, PyDict, PyList, PyListMethods, PyModule},
};
use serde_json::Value;

use super::RawToolDef;
use crate::ToolError;

/// Load all `@tool`-decorated functions from `*.py` files in `dir`.
pub(crate) fn load(dir: &Path) -> Result<Vec<RawToolDef>, ToolError> {
    if !dir.is_dir() {
        return Err(ToolError::Runtime(format!(
            "Python adapter: path is not a directory: {}",
            dir.display()
        )));
    }

    let mut defs = Vec::new();

    let all_entries: Vec<fs::DirEntry> = fs::read_dir(dir)
        .map_err(|e| {
            ToolError::Runtime(format!(
                "Python adapter: failed to read directory {}: {e}",
                dir.display()
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            ToolError::Runtime(format!(
                "Python adapter: failed to iterate directory {}: {e}",
                dir.display()
            ))
        })?;

    let entries: Vec<_> = all_entries
        .into_iter()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "py"))
        .collect();

    if entries.is_empty() {
        return Ok(defs);
    }

    Python::with_gil(|py| {
        let saved_path = save_sys_path(py)?;

        let _ = add_to_sys_path(py, &dir.to_string_lossy());

        if let Some(sp) = find_venv_site_packages(dir) {
            let _ = add_to_sys_path(py, &sp);
        }

        let result: Result<(), ToolError> = (|| {
            for entry in &entries {
                let file_path = entry.path();
                let source = fs::read_to_string(&file_path).map_err(|e| {
                    ToolError::Runtime(format!(
                        "Python adapter: failed to read {}: {e}",
                        file_path.display()
                    ))
                })?;
                let file_defs = load_file(py, &file_path, &source)?;
                defs.extend(file_defs);
            }
            Ok(())
        })();

        let _ = restore_sys_path(py, &saved_path);
        result?;
        Ok(defs)
    })
}

/// Load tools from a single Python file.
fn load_file(py: Python<'_>, path: &Path, source: &str) -> Result<Vec<RawToolDef>, ToolError> {
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

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
    );

    let c_source = CString::new(full_source)
        .map_err(|_| ToolError::Runtime("Python source contains null bytes".into()))?;
    let c_file = CString::new(format!("{file_name}.py"))
        .map_err(|_| ToolError::Runtime("file name contains null bytes".into()))?;
    let c_module = CString::new(file_name)
        .map_err(|_| ToolError::Runtime("module name contains null bytes".into()))?;

    let module = PyModule::from_code(py, &c_source, &c_file, &c_module).map_err(|e| {
        ToolError::Runtime(format!(
            "Python adapter: failed to execute {}: {e}",
            path.display()
        ))
    })?;

    let mut defs = Vec::new();
    let dir_list = module
        .dir()
        .map_err(|e| ToolError::Runtime(format!("Python adapter: failed to list module attrs: {e}")))?;

    for attr_name in dir_list.iter() {
        let attr_name_str: String = attr_name
            .extract()
            .map_err(|e| ToolError::Runtime(format!("Python adapter: failed to extract attr name: {e}")))?;

        if attr_name_str.starts_with('_') {
            continue;
        }

        let obj = match module.getattr(&*attr_name_str) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let tool_dict = match obj.getattr("__tool__") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let def = extract_tool_def(py, &tool_dict, obj)?;
        defs.push(def);
    }

    Ok(defs)
}

/// Extract a [`RawToolDef`] from a function's `__tool__` dict.
fn extract_tool_def(
    py: Python<'_>,
    tool_dict: &Bound<'_, PyAny>,
    callable: Bound<'_, PyAny>,
) -> Result<RawToolDef, ToolError> {
    let name: String = tool_dict
        .get_item("name")
        .map_err(|e| ToolError::Runtime(format!("missing 'name' in __tool__: {e}")))?
        .extract()
        .map_err(|e| ToolError::Runtime(format!("'name' is not a string: {e}")))?;

    let description: String = tool_dict
        .get_item("description")
        .map_err(|e| ToolError::Runtime(format!("missing 'description' in __tool__: {e}")))?
        .extract()
        .map_err(|e| ToolError::Runtime(format!("'description' is not a string: {e}")))?;

    let params_obj = tool_dict
        .get_item("parameters")
        .map_err(|e| ToolError::Runtime(format!("missing 'parameters' in __tool__: {e}")))?;
    let parameters = py_to_json(py, &params_obj)?;

    let meta_obj = tool_dict
        .get_item("meta")
        .map_err(|e| ToolError::Runtime(format!("missing 'meta' in __tool__: {e}")))?;
    let meta = py_to_json(py, &meta_obj)?;

    let call_fn = Arc::new(callable.unbind());

    let func: Box<
        dyn Fn(Value) -> futures::future::BoxFuture<'static, Result<Value, ToolError>>
            + Send
            + Sync,
    > = Box::new(move |args: Value| {
        let call_fn = call_fn.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                Python::with_gil(|py| {
                    let kwargs = json_to_py(py, &args).map_err(|e| {
                        ToolError::Runtime(format!("failed to convert args to Python: {e}"))
                    })?;
                    let kwargs_dict = kwargs.downcast_bound::<PyDict>(py).map_err(|e| {
                        ToolError::Runtime(format!("args must be an object: {e}"))
                    })?;

                    let result = call_fn.call(py, (), Some(kwargs_dict)).map_err(|e| {
                        ToolError::Runtime(format!("Python tool error: {e}"))
                    })?;

                    py_to_json(py, result.bind(py))
                })
            })
            .await
            .map_err(|e| ToolError::Runtime(format!("spawn_blocking failed: {e}")))?
        })
    });

    Ok(RawToolDef {
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

fn py_to_json(py: Python<'_>, obj: &Bound<'_, PyAny>) -> Result<Value, ToolError> {
    let json_mod = py
        .import("json")
        .map_err(|e| ToolError::Runtime(format!("failed to import json: {e}")))?;
    let json_str: String = json_mod
        .call_method1("dumps", (obj,))
        .map_err(|e| ToolError::Runtime(format!("json.dumps failed: {e}")))?
        .extract()
        .map_err(|e| ToolError::Runtime(format!("json.dumps didn't return str: {e}")))?;
    serde_json::from_str(&json_str)
        .map_err(|e| ToolError::Runtime(format!("invalid JSON from Python: {e}")))
}

fn json_to_py(py: Python<'_>, value: &Value) -> Result<PyObject, String> {
    let json_str =
        serde_json::to_string(value).map_err(|e| format!("failed to serialize JSON: {e}"))?;
    let json_mod = py
        .import("json")
        .map_err(|e| format!("failed to import json: {e}"))?;
    let obj = json_mod
        .call_method1("loads", (json_str,))
        .map_err(|e| format!("json.loads failed: {e}"))?;
    Ok(obj.unbind())
}

// ============================================================================
// VENV DETECTION
// ============================================================================

fn find_venv_site_packages(dir: &Path) -> Option<String> {
    let mut search = Some(dir);
    while let Some(d) = search {
        let venv = d.join(".venv");
        if venv.is_dir() {
            return find_site_packages(&venv).ok();
        }
        search = d.parent();
    }
    None
}

fn find_site_packages(venv: &Path) -> Result<String, String> {
    // Windows: Lib/site-packages
    let win_sp = venv.join("Lib").join("site-packages");
    if win_sp.is_dir() {
        return win_sp
            .to_str()
            .map(|s| s.to_string())
            .ok_or("non-UTF8 path".into());
    }

    // Unix: lib/pythonX.Y/site-packages
    let lib = venv.join("lib");
    if !lib.is_dir() {
        return Err("no lib/ in venv".into());
    }
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

// ============================================================================
// SYS.PATH MANAGEMENT
// ============================================================================

fn save_sys_path(py: Python<'_>) -> Result<PyObject, ToolError> {
    let sys = py
        .import("sys")
        .map_err(|e| ToolError::Runtime(format!("failed to import sys: {e}")))?;
    let path = sys
        .getattr("path")
        .map_err(|e| ToolError::Runtime(format!("failed to get sys.path: {e}")))?;
    let copy = path
        .call_method0("copy")
        .map_err(|e| ToolError::Runtime(format!("failed to copy sys.path: {e}")))?;
    Ok(copy.unbind())
}

fn restore_sys_path(py: Python<'_>, saved: &PyObject) -> Result<(), ToolError> {
    let sys = py
        .import("sys")
        .map_err(|e| ToolError::Runtime(format!("failed to import sys: {e}")))?;
    sys.setattr("path", saved.bind(py))
        .map_err(|e| ToolError::Runtime(format!("failed to restore sys.path: {e}")))?;
    Ok(())
}

fn add_to_sys_path(py: Python<'_>, path: &str) -> Result<(), ToolError> {
    let sys = py
        .import("sys")
        .map_err(|e| ToolError::Runtime(format!("failed to import sys: {e}")))?;
    let sys_path = sys
        .getattr("path")
        .map_err(|e| ToolError::Runtime(format!("failed to get sys.path: {e}")))?;
    let sys_path = sys_path
        .downcast_into::<PyList>()
        .map_err(|e| ToolError::Runtime(format!("sys.path is not a list: {e}")))?;
    sys_path
        .insert(0, path)
        .map_err(|e| ToolError::Runtime(format!("failed to insert into sys.path: {e}")))?;
    Ok(())
}
