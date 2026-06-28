from __future__ import annotations

import importlib.util
import importlib
import sys
from pathlib import Path


def _load_native_module():
	try:
		native_module = importlib.import_module(f"{__name__}.airllm")

		return native_module
	except ModuleNotFoundError:
		pass

	package_dir = Path(__file__).resolve().parent
	workspace_root = package_dir.parents[1]
	candidates = []
	for profile in ("debug", "release"):
		target_dir = workspace_root / "target" / profile
		candidates.extend(target_dir.glob("libairllm_python*.so"))
		candidates.extend(target_dir.glob("airllm_python*.so"))
		candidates.extend(target_dir.glob("libairllm*.so"))
		candidates.extend(target_dir.glob("airllm*.so"))

	for candidate in candidates:
		spec = importlib.util.spec_from_file_location(f"{__name__}.airllm", candidate)
		if spec is None or spec.loader is None:
			continue

		module = importlib.util.module_from_spec(spec)
		sys.modules[spec.name] = module
		spec.loader.exec_module(module)
		return module

	raise ModuleNotFoundError(
		"Could not import the native 'airllm' extension. Build the Rust bindings first "
		"with cargo build -p airllm-python or install the package with the compiled module."
	)


Orchestrator = _load_native_module().Orchestrator

__all__ = ["Orchestrator"]
