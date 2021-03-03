use std::path::PathBuf;

use std::sync::{Arc, Mutex};

pub trait BazelInfo {
	fn get_exec_root(&self, source_root: &PathBuf) -> Result<PathBuf, String>;
	fn debug(&self) -> String;
}
impl std::fmt::Debug for dyn BazelInfo + Send + Sync {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BazelInfo({})", self.debug())
    }
}

#[derive(Debug, Default, Clone)]
struct BazelExecutable {
  executable: PathBuf,
}
impl BazelExecutable {
	pub fn new(executable: &str) -> Self {
        BazelExecutable{executable: PathBuf::from(executable)}
	}

	fn call_bazel(&self, command: Vec<String>, cwd: &PathBuf) -> Result<String, String> {
		std::process::Command::new(&self.executable)
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
}
impl BazelInfo for BazelExecutable {
    fn get_exec_root(&self, source_root: &PathBuf) -> Result<PathBuf, String> {
		let execroot: String = self.call_bazel(
			vec!["info".to_string(), "execution_root".to_string()],
			source_root,
		)?;
		Ok(PathBuf::from(
			execroot.replace("execroot/__main__", "external"),
		))
	}

	fn debug(&self) -> String {
		format!("{:?}", self)
	}
}

#[derive(Debug)]
pub struct BazelWorkspace {
	inner: Arc<Mutex<InnerBazel>>,
}

impl BazelWorkspace {
	pub fn new() -> Self {
		BazelWorkspace {
			inner: Arc::new(Mutex::new(InnerBazel::new())),
		}
	}

	pub fn with_bazel_info(bazel_info: Box<dyn BazelInfo + Send + Sync>) -> Self {
		BazelWorkspace {
			inner: Arc::new(Mutex::new(InnerBazel::with_bazel_info(bazel_info))),
		}
	}

	pub fn maybe_change_source_root(&self, new_root: &PathBuf) -> Result<(), String> {
		let inner = &mut *self
			.inner
			.lock()
			.map_err(|err| format!("Error locking Bazel {:?}", err))?;
		inner.maybe_change_source_root(new_root)
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
	workspace_root: Option<PathBuf>,
	source_root: Option<PathBuf>, // Where to resolve "//:" references against
	bazel_info: Box<dyn BazelInfo + Send + Sync>,
}

impl InnerBazel {
	pub fn new() -> Self {
		InnerBazel {
			exec_root: None,
			workspace_root: None,
			source_root: None,
			bazel_info: Box::new(BazelExecutable::new("bazelisk")),
		}
	}

	pub fn with_bazel_info(bazel_info: Box<dyn BazelInfo + Send + Sync>) -> Self {
		InnerBazel {
			exec_root: None,
			workspace_root: None,
			source_root: None,
			bazel_info: bazel_info,
		}
	}

	pub	fn maybe_change_source_root(&mut self, file_path: &PathBuf) -> Result<(), String> {
		if self.source_root.is_none() || self.workspace_root.is_none() {
			Err(format!("Trying to change root to {:?}, but Bazel is not initialized!", file_path))
		} else {
			let exec_root = self.exec_root.as_ref().unwrap();
			let workspace_root = self.workspace_root.as_ref().unwrap();
			if file_path.starts_with(&exec_root) {
				let ancestors = file_path.ancestors();
				let mut new_root = None;
				for ancestor in ancestors.take_while(|anc| anc != exec_root) {
					new_root = Some(ancestor);
				}
				self.source_root = new_root.map(|nr| PathBuf::from(nr));
			// }
			// if file_path.ancestors().find(|anc| anc == &exec_root).is_some() {
			// 	self.source_root = self.exec_root.clone();
			} else if file_path.ancestors().find(|anc| anc == &workspace_root).is_some() {
				self.source_root = self.workspace_root.clone();
			}
			Ok(())
		}
	}

	pub fn update_workspace(&mut self, workspace: &PathBuf) -> Result<(), String> {
		self.bazel_info.get_exec_root(workspace).map(|root| {
			self.source_root = Some(workspace.clone());
			self.workspace_root = self.source_root.clone();
			self.exec_root = Some(root);
			()
		})
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
