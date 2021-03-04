use std::path::PathBuf;
use rustpython_parser::ast;

use crate::index::range::Range;

#[derive(Debug, Clone, PartialEq)]
pub enum CallableSymbolSource {
	DeclaredInFile(Range),
	Loaded(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
	pub imported_name: String,
	pub real_name: String,
	pub source: CallableSymbolSource,
}

impl FunctionDecl {
	pub fn declared_in_file(name: &String, location: ast::Location) -> Self {
		// We account for the "def " keyword here, which the parser doesn't pick up on.
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
}
