use std::path::PathBuf;

#[derive(Debug)]
pub struct Bazel {
	exec_root: Option<PathBuf>,
}

impl Bazel {
	pub fn new() -> Self {
		Bazel {
			exec_root: None,
		}
	}

	pub fn update_workspace(&mut self, workspace: &PathBuf) -> Result<(), String> {
		self.call_bazel(
			vec![
				"sync".to_string(),
			],
			workspace,
		)?;
		self.get_exec_root(workspace).map(|root| {self.exec_root = Some(root); ()})
	}

	fn get_exec_root(&self, source_root: &PathBuf) -> Result<PathBuf, String> {
		let execroot: String = self.call_bazel(
			vec!["info".to_string(), "execution_root".to_string()],
			source_root,
		)?;
		Ok(PathBuf::from(execroot.replace("execroot/__main__", "external")))
	}

	fn call_bazel(&self, command: Vec<String>, cwd: &PathBuf) -> Result<String, String> {
		std::process::Command::new("bazel")
			.args(&command)
			.current_dir(cwd)
			.output()
			.map_err(|err| format!("Error running Bazel command {:?}: {:?}", command, err))
			.and_then(|out| {
				String::from_utf8(out.stdout)
					.map_err(|err| format!("Error parsing output: {:?}", err))
			})
	}

	pub fn resolve_bazel_path(&self, path: &String) -> Result<PathBuf, String> {
		if path.starts_with("//") {
			let resolved_path = std::env::current_dir()
						.expect("Error getting current dir.")
						.as_os_str()
						.to_str()
						.expect("Error converting current dir to string")
						.to_owned() + "/" + path.strip_prefix("//:").unwrap();

			Ok(PathBuf::from(resolved_path))
		} else {
			let resolved_path = path.replace("@", "").replace("//", "").replace(":", "/");
			self
				.exec_root
				.as_ref()
				.ok_or_else(|| format!("Empty exec_root!"))
				.map(|root| {
					let mut res = root.clone();
					res.push(PathBuf::from(resolved_path));
					res
				})
			}
	}
}
