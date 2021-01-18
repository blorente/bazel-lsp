use std::fs::read_to_string;
use std::path::PathBuf;

use rustpython_parser::{parser, ast};
use tower_lsp::lsp_types::{DocumentHighlight, DocumentHighlightKind};
use tower_lsp::lsp_types;

pub fn highlight(file: &PathBuf) -> Result<Option<Vec<DocumentHighlight>>, ()> {
	let content: String = read_to_string(file).map_err(|_| ())?;
	let program: ast::Program = parser::parse_program(&content).map_err(|_| ())?;
	let highlights: Vec<DocumentHighlight> = program.statements.iter().flat_map(|stmt| highlight_statement(stmt)).collect::<_>();
	if highlights.is_empty() {
		Ok(None)
	} else {
		Ok(Some(highlights))
	}
}

fn highlight_statement(statement: &ast::Statement) -> Vec<DocumentHighlight> {
	let mut highlights = vec![];
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef {name, ..} => highlights.push(DocumentHighlight{
			range: loc_to_range(location, &name),
			kind: Some(DocumentHighlightKind::Text),
		}),
		_ => {},
	};
	highlights
}

fn loc_to_range(location: ast::Location, name: &str) -> lsp_types::Range {
	let row = location.row() as u64;
	let col = location.column() as u64;

	lsp_types::Range::new(
		lsp_types::Position {
			line: row,
			character: col,
		},
		lsp_types::Position {
			line: row,
			character: col + name.len() as u64,
		}
	)
}
