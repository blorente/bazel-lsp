use std::path::PathBuf;
use tower_lsp::lsp_types as lsp;
use rustpython_parser::ast;

use crate::range::Range;

#[derive(Debug, Clone)]
pub enum CallableSymbolSource {
	Stdlib,
	DeclaredInFile(Range),
	Loaded(PathBuf),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
	imported_name: String,
	real_name: String,
	source: CallableSymbolSource,
}

impl FunctionDecl {
	pub fn declared_in_file(name: &String, location: ast::Location) -> Self {
		// We account for the "def " keyword here, which the parser doesn't pick up on.
		let location = ast::Location::new(location.row(), location.column() + "def ".len());
		FunctionDecl {
			imported_name: name.clone(),
			real_name: name.clone(),
			source: CallableSymbolSource::DeclaredInFile(Range::from_identifier(name, location)),
		}
	}

	pub fn loaded(name: &String, imported_name: &String, source: &PathBuf) -> Self {
		FunctionDecl {
			imported_name: imported_name.clone(),
			real_name: name.clone(),
			source: CallableSymbolSource::Loaded(source.clone()),
		}
	}

	pub fn lsp_location(&self, current_file: &lsp::Url) -> lsp::Location {
		match &self.source {
			CallableSymbolSource::DeclaredInFile(range) => {
				lsp::Location::new(current_file.clone(), range.as_lsp_range())
			}
			CallableSymbolSource::Loaded(source) => {
				let zero_zero = lsp::Position::new(0, 0);
				lsp::Location::new(
					lsp::Url::from_file_path(&source).expect(&format!(
						"Failed to convert path {:?} to URL",
						&source,
					)),
					lsp::Range::new(zero_zero, zero_zero),
				)
			}
			_ => panic!("Unimplemented"),
		}
	}
}
