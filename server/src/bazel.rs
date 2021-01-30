use std::path::PathBuf;

#[derive(Debug)]
pub struct Bazel {
	distdir: PathBuf,
}

impl Bazel {
	pub fn new(distdir: PathBuf) -> Self {
		Bazel { distdir }
	}

	pub fn update_workspace(&self, workspace: &PathBuf) -> Result<(), String> {
		let distdir_as_string = self.distdir.clone().into_os_string().into_string().expect("");
		self.call_bazel(vec![
			"sync".to_string(),
			String::from("--distdir=") + &distdir_as_string,
		], workspace)
	}

	fn call_bazel(&self, command: Vec<String>, cwd: &PathBuf) -> Result<(), String> {
		std::process::Command::new("bazel")
			.args(&command)
			.current_dir(cwd)
			.output()
			.map(|_| ())
			.map_err(|err| format!("Error running Bazel command {:?}: {:?}", command, err))
	}
}
