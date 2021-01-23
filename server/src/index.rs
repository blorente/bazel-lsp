use rustpython_parser::ast;
use rustpython_parser::parser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::fs::read_to_string;
use tower_lsp::lsp_types as lsp;

pub fn parse(file: &PathBuf) -> Result<ast::Program, ()> {
       let content: String = read_to_string(file).map_err(|_| ())?;
       parser::parse_program(&content).map_err(|_| ())
}

#[derive(Debug, Clone)]
pub enum CallableSymbolSource {
	Stdlib,
	DeclaredInFile(Range),
	Loaded(String, PathBuf),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
	imported_name: String,
	real_name: String,
	source: CallableSymbolSource,
}

impl FunctionDecl {
	fn declared_in_file(name: &String, location: ast::Location) -> Self {
		// We account for the "def " keyword here, which the parser doesn't pick up on.
		let location = ast::Location::new(location.row(), location.column() + "def ".len());
		FunctionDecl {
			imported_name: name.clone(),
			real_name: name.clone(),
			source: CallableSymbolSource::DeclaredInFile(Range::from_identifier(name, location)),
		}
	}

	pub fn lsp_location(&self, current_file: &tower_lsp::lsp_types::Url) -> lsp::Location {
		let range = match &self.source {
			CallableSymbolSource::DeclaredInFile(range) => range.as_lsp_range(),
			_ => panic!("Unimplemented"),
		};
		lsp::Location::new(current_file.clone(), range)
	}
}

fn ast_location_to_lsp_position(location: ast::Location) -> lsp::Position {
	// Lsp positions are 0-based, whereas parser positions are 1-based,
	lsp::Position::new(location.row() as u64 - 1, location.column() as u64 - 1)
}

#[derive(Debug, Clone)]
pub struct Range {
	start: lsp::Position,
	end: lsp::Position,
}

impl Range {
	pub fn from_identifier(name: &String, location: ast::Location) -> Self {
		let start =	ast_location_to_lsp_position(location);
		let end = lsp::Position::new(start.line, start.character + name.len() as u64);
		Range {	start, end }
	}

	pub fn as_lsp_range(&self) -> lsp::Range {
		lsp::Range::new(
			self.start.clone(),
			self.end.clone(),
		)
	}

	pub fn contains_position(&self, position: lsp::Position) -> bool {
		self.start <= position && self.end >= position
	}
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
	range: Range,
	function_name: String,
}

impl FunctionCall {
	fn from_identifier(name: &String, location: ast::Location) -> Self {
		FunctionCall {
			range: Range::from_identifier(name, location),
			function_name: name.clone(),
		}
	}

	fn contains_position(&self, position: lsp::Position) -> bool {
		self.range.contains_position(position)
	}
}

#[derive(Default, Debug)]
pub struct Documents {
	docs: Mutex<HashMap<PathBuf, DocumentIndex>>,
}

impl Documents {
	pub fn refresh_doc(&self, doc: &PathBuf) {
		let index = &mut *self.docs.lock().expect("");
		let index_for_doc = DocumentIndex::index_document(doc).expect("");
		index.insert(doc.clone(), index_for_doc);
	}

	pub fn get_doc(&self, doc: &PathBuf) -> Option<DocumentIndex> {
		let docs = &*self.docs.lock().expect("Failed to lock");
		// TODO This clone could get very expensive, we should wrap indexes in Arcs
		docs.get(doc).cloned()
	}
}

#[derive(Default, Debug, Clone)]
pub struct DocumentIndex {
	declarations: HashMap<String, FunctionDecl>,
	calls: Vec<FunctionCall>,
}

fn process_statement(index: &mut DocumentIndex, statement: &ast::Statement) {
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef { name, body, .. } => {
			index.declarations.insert(
				name.clone(),
				FunctionDecl::declared_in_file(name, location),
			);
			process_suite(index, body);
		}
		ast::StatementType::Expression { expression } => match &expression.node {
			ast::ExpressionType::Call { function, .. } => match &function.node {
				ast::ExpressionType::Identifier { name, .. } => index
					.calls
					.push(FunctionCall::from_identifier(name, function.location)),
				_ => {}
			},
			_ => {}
		},
		_ => {}
	};
}

fn process_suite(index: &mut DocumentIndex, suite: &ast::Suite) {
	for stmt in suite {
		process_statement(index, &stmt);
	}
}

impl DocumentIndex {
	pub fn index_document(path: &PathBuf) -> Result<Self, ()> {
		let ast = parse(path)?;
		let mut index = DocumentIndex::default();
		process_suite(&mut index, &ast.statements);
		Ok(index)
	}

	// TODO This should probably live in a new struct to represent all calls
	pub fn call_at(&self, position: lsp::Position) -> Option<FunctionCall> {
		self.calls
			.iter()
			.find(|call| call.contains_position(position))
			.cloned()
	}

	pub fn declaration_of(&self, call: &FunctionCall) -> Option<FunctionDecl> {
		self.declarations.get(&call.function_name).cloned()
	}
}
