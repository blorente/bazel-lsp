use std::collections::HashMap;
use std::path::PathBuf;
use tower_lsp::lsp_types as lsp;

use crate::index::function_decl::FunctionDecl;
use crate::index::function_call::FunctionCall;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct IndexedDocument {
	pub declarations: HashMap<String, FunctionDecl>,
	pub calls: Vec<FunctionCall>,
}

impl IndexedDocument {
	pub fn new() -> Self {
		IndexedDocument {
			declarations: HashMap::default(),
			calls: Vec::default(),
		}
	}

    pub fn finished(declarations: HashMap<String, FunctionDecl>, calls: Vec<FunctionCall>) -> Self {
		IndexedDocument {
			declarations, calls
		}
	}

	// TODO This should probably live in a new struct to represent all calls
	pub fn call_at(&self, position: lsp::Position) -> Option<FunctionCall> {
		self.calls
			.iter()
			.find(|call| call.contains_position(position))
			.cloned()
	}

	pub fn declaration_of(&self, name: &String) -> Option<FunctionDecl> {
		self.declarations.get(name).cloned()
	}
}