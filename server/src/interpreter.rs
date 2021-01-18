use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use starlark::eval::{EvalException, FileLoader};
use starlark::environment::{TypeValues, Environment};
use starlark::syntax::dialect::Dialect;
use starlark::stdlib::global_environment;
use codemap::CodeMap;


#[derive(Debug)]
pub struct Starlark {
	codemap: Arc<Mutex<CodeMap>>,
	environment: Environment,
	type_values: TypeValues,
}

impl Starlark {
	pub fn new() -> Self {
		let (global_env, type_values) = global_environment();
		Starlark {
				codemap: Arc::new(Mutex::new(codemap::CodeMap::new())),
				environment: global_env,
				type_values: type_values,
		}
		
	}

	pub fn run(&mut self, file: &PathBuf, file_loader: &dyn FileLoader) -> Result<Environment, ()> {
		let mut env = self.environment.child("eval");
		starlark::eval::eval_file(
			&self.codemap,
			file.to_str().ok_or(())?,
			Dialect::Bzl,
			&mut env,
			&self.type_values,
			file_loader,
		).map_err(|_|())?;
		Ok(env)
	}
}


#[derive(Debug)]
pub struct BazelWorkspaceLoader {
	pub workspace: Option<Box<PathBuf>>,
}

impl FileLoader for BazelWorkspaceLoader {
	fn load(
		&self,
		path: &str,
		type_values: &TypeValues
	) -> Result<Environment, EvalException> {
		panic!();
	}
	
}