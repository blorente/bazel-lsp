
use tower_lsp::lsp_types as lsp;
use rustpython_parser::ast;

fn ast_location_to_lsp_position(location: ast::Location) -> lsp::Position {
	// Lsp positions are 0-based, whereas parser positions are 1-based,
	lsp::Position::new(location.row() as u64 - 1, location.column() as u64 - 1)
}
#[derive(Debug, Clone, PartialEq)]
pub struct Range {
	start: lsp::Position,
	end: lsp::Position,
}

impl Range {
	pub fn from_identifier(name: &String, location: ast::Location) -> Self {
		let start = ast_location_to_lsp_position(location);
		let end = lsp::Position::new(start.line, start.character + name.len() as u64);
		Range { start, end }
	}

	pub fn as_lsp_range(&self) -> lsp::Range {
		lsp::Range::new(self.start.clone(), self.end.clone())
	}

	pub fn contains_position(&self, position: lsp::Position) -> bool {
		self.start <= position && self.end >= position
	}
}