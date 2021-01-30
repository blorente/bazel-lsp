use std::path::PathBuf;

use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Bazel {
	inner: Arc<Mutex<InnerBazel>>,
}

impl Bazel {
	pub fn new() -> Self {
		Bazel {
			inner: Arc::new(Mutex::new(InnerBazel::new())),
		}
	}

	pub fn update_exec_root(&self, workspace: &PathBuf) -> Result<(), String> {
		let inner = &mut *self
			.inner
			.lock()
			.map_err(|err| format!("Error locking Bazel {:?}", err))?;
		inner.update_exec_root(workspace)
	}
	pub fn update_workspace(&self, workspace: &PathBuf) -> Result<(), String> {
		let inner = &mut *self
			.inner
			.lock()
			.map_err(|err| format!("Error locking Bazel {:?}", err))?;
		inner.update_workspace(workspace)
	}
	pub fn resolve_bazel_path(&self, path: &String) -> Result<PathBuf, String> {
		let inner = &*self
			.inner
			.lock()
			.map_err(|err| format!("Error locking Bazel {:?}", err))?;
		inner.resolve_bazel_path(path)
	}
}

#[derive(Debug)]
struct InnerBazel {
	exec_root: Option<PathBuf>,
	source_root: Option<PathBuf>,
}

impl InnerBazel {
	pub fn new() -> Self {
		InnerBazel {
			exec_root: None,
			source_root: None,
		}
	}

	pub fn update_exec_root(&mut self, workspace: &PathBuf) -> Result<(), String> {
		self.get_exec_root(workspace).map(|root| {
			self.source_root = Some(workspace.clone());
			self.exec_root = Some(root);
			()
		})
	}

	pub fn update_workspace(&mut self, workspace: &PathBuf) -> Result<(), String> {
		self.call_bazel(vec!["sync".to_string()], workspace)?;
		self.update_exec_root(workspace)
	}

	fn get_exec_root(&self, source_root: &PathBuf) -> Result<PathBuf, String> {
		let execroot: String = self.call_bazel(
			vec!["info".to_string(), "execution_root".to_string()],
			source_root,
		)?;
		Ok(PathBuf::from(
			execroot.replace("execroot/__main__", "external"),
		))
	}

	fn call_bazel(&self, command: Vec<String>, cwd: &PathBuf) -> Result<String, String> {
		std::process::Command::new("bazelisk")
			.args(&command)
			.current_dir(cwd)
			.output()
			.map_err(|err| format!("Error running Bazel command {:?}: {:?}", command, err))
			.and_then(|out| {
				String::from_utf8(out.stdout)
					.map_err(|err| format!("Error parsing output: {:?}", err))
			})
			.map(|out| out.trim().to_string())
	}

	fn sanitize_starlark_label(&self, label: &String) -> String {
		let label = label
			.trim()
			.replace("@", "")
			.replace("//:", "/")
			.replace("//", "/")
			.replace(":", "/");
		if label.starts_with("/") {
			label.strip_prefix("/").unwrap().to_string()
		} else {
			label
		}
		
	}

	pub fn resolve_bazel_path(&self, path: &String) -> Result<PathBuf, String> {
		let resolved_path = self.sanitize_starlark_label(path);
		let maybe_root = if path.starts_with("//") {
			self.source_root.as_ref()
		} else {
			self.exec_root.as_ref()
		};
		maybe_root
			.ok_or_else(|| format!("Empty source_root!"))
			.and_then(|root| {
				let mut res = root.clone();
				res.push(PathBuf::from(resolved_path));
				if res.is_file() {
					Ok(res)
				} else {
					let err = format!("Resolved file {:?} from {}, but file doesn't exist!", res, path);
					Err(err)
				}
			})
	}
}
