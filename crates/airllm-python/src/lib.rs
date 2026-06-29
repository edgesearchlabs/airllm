#![allow(clippy::useless_conversion)]

use airllm_ollama::OllamaClient;
use airllm_orchestrator::{CodeRequest, Orchestrator};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Python-facing orchestrator wrapper.
#[pyclass(name = "Orchestrator")]
struct PyOrchestrator {
	inner: Orchestrator,
}

#[pymethods]
impl PyOrchestrator {
	#[new]
	#[pyo3(signature = (ollama_url=None))]
	fn new(ollama_url: Option<String>) -> PyResult<Self> {
		let url = ollama_url.unwrap_or_else(|| "http://localhost:11434".to_string());
		Ok(Self {
			inner: Orchestrator::new(OllamaClient::new(&url)),
		})
	}

	#[allow(clippy::useless_conversion)]
	#[pyo3(signature = (task, language=None))]
	fn code(&self, task: &str, language: Option<&str>) -> PyResult<String> {
		let req = CodeRequest {
			task: task.to_string(),
			language: language.map(|s| s.to_string()),
			files: Vec::new(),
			model_override: None,
			permission_mode: "bypass".to_string(),
			max_rounds: 5,
		};
		let rt = tokio::runtime::Runtime::new()?;
		let resp = rt.block_on(self.inner.code(req)).map_err(to_py_err)?;
		Ok(resp.output)
	}

	#[allow(clippy::useless_conversion)]
	fn list_models(&self) -> PyResult<Vec<String>> {
		let rt = tokio::runtime::Runtime::new()?;
		let resp = rt.block_on(self.inner.list_models()).map_err(to_py_err)?;
		Ok(resp)
	}

	#[pyo3(signature = (models=None))]
	fn prewarm_models(&self, models: Option<Vec<String>>) -> PyResult<Vec<String>> {
		let rt = tokio::runtime::Runtime::new()?;
		let warmed = rt.block_on(self.inner.prewarm_models(models)).map_err(to_py_err)?;
		Ok(warmed)
	}
}

#[pymodule]
fn airllm(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
	m.add_class::<PyOrchestrator>()?;
	Ok(())
}

fn to_py_err<E: std::fmt::Display>(err: E) -> PyErr {
	PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{err}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn smoke_imports() {
		Python::with_gil(|py| {
			let module = PyModule::new_bound(py, "airllm").unwrap();
			airllm(py, &module).unwrap();
		});
	}
}