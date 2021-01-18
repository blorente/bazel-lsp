use std::fs::read_to_string;
use std::path::PathBuf;

use rustpython_parser::{ast, parser};
use tower_lsp::lsp_types;
use tower_lsp::lsp_types::{
	DocumentHighlight, DocumentHighlightKind, Location, SymbolInformation, SymbolKind, Url,
};

fn parse(file: &PathBuf) -> Result<ast::Program, ()> {
	let content: String = read_to_string(file).map_err(|_| ())?;
	parser::parse_program(&content).map_err(|_| ())
}

fn extract<T, F>(file: &PathBuf, process_statment: F) -> Result<Option<Vec<T>>, ()>
where
	F: Fn(&ast::Statement, &PathBuf) -> Vec<T>,
{
	let program = parse(file)?;
	let results: Vec<T> = program
		.statements
		.iter()
		.flat_map(|stmt| process_statment(stmt, file))
		.collect::<Vec<T>>();
	if results.is_empty() {
		Ok(None)
	} else {
		Ok(Some(results))
	}
}

pub fn highlight(file: &PathBuf) -> Result<Option<Vec<DocumentHighlight>>, ()> {
	extract(file, |stmt, _| highlight_statement(&stmt))
}

fn highlight_statement(statement: &ast::Statement) -> Vec<DocumentHighlight> {
	let mut highlights = vec![];
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef { name, .. } => highlights.push(DocumentHighlight {
			range: loc_to_range(location, &name),
			kind: Some(DocumentHighlightKind::Text),
		}),
		_ => {}
	};
	highlights
}

pub fn extract_symbols(file: &PathBuf) -> Result<Option<Vec<SymbolInformation>>, ()> {
	extract(file, |stmt, path| {
		extract_symbols_from_statement(&stmt, path)
	})
}

fn extract_symbols_from_statement(
	statement: &ast::Statement,
	path: &PathBuf,
) -> Vec<SymbolInformation> {
	let mut symbols = vec![];
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef { name, .. } => symbols.push(SymbolInformation {
			name: name.clone(),
			kind: SymbolKind::Function,
			deprecated: None,
			location: Location::new(Url::from_file_path(path).expect("Something went very wrong"), loc_to_range(location, name)),
			container_name: None,
		}),
		_ => {}
	};
	symbols
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
		},
	)
}
