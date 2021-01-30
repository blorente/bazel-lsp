use tower_lsp::lsp_types as lsp;
use rustpython_parser::ast;

use crate::index::range::Range;

#[derive(Debug, Clone)]
pub struct FunctionCall {
	range: Range,
	pub function_name: String,
}

impl FunctionCall {
	pub fn from_identifier(name: &String, location: ast::Location) -> Self {
		FunctionCall {
			range: Range::from_identifier(name, location),
			function_name: name.clone(),
		}
	}

	pub fn contains_position(&self, position: lsp::Position) -> bool {
		self.range.contains_position(position)
	}
}