use std::path::PathBuf;

use eyre::eyre;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub(crate) struct BazelExecutable {
    executable: PathBuf,
}
impl BazelExecutable {
    pub(crate) fn new(executable: &str) -> Self {
        BazelExecutable {
            executable: PathBuf::from(executable),
        }
    }

    pub(crate) fn call_bazel(&self, command: Vec<String>, cwd: &PathBuf) -> eyre::Result<String> {
        let output = std::process::Command::new(&self.executable)
            .args(&command)
            .current_dir(cwd)
            .output()
            .map_err(|err| eyre!("Error running Bazel command {:?}: {:?}", command, err))?;
        let as_str = String::from_utf8(output.stdout)
            .map_err(|err| eyre!("Error parsing output: {:?}", err))?;
        Ok(as_str.trim().to_string())
    }
}
