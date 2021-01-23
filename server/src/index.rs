use rustpython_parser::ast;
use rustpython_parser::parser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::fs::read_to_string;
use tower_lsp::lsp_types::Location as LspLocation;
use tower_lsp::lsp_types::Position as LspPosition;
use tower_lsp::lsp_types::Range as LspRange;

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
pub struct CallableSymbol {
	imported_name: String,
	real_name: String,
	source: CallableSymbolSource,
}

impl CallableSymbol {
	fn declared_in_file(name: &String, location: ast::Location) -> Self {
		CallableSymbol {
			imported_name: name.clone(),
			real_name: name.clone(),
			source: CallableSymbolSource::DeclaredInFile(Range::from_identifier(name, location)),
		}
	}

	pub fn lsp_location(&self, current_file: &tower_lsp::lsp_types::Url) -> LspLocation {
		let range = match &self.source {
			CallableSymbolSource::DeclaredInFile(range) => range.as_lsp_range(),
			_ => panic!("Unimplemented"),
		};
		LspLocation::new(current_file.clone(), range)
	}
}

#[derive(Debug, Clone)]
pub struct Range {
	start: ast::Location,
	end: ast::Location,
}

// TODO Lsp positions are 0-based, parser positions are 1-based.
// For convenience, we should ditch this data structure and store things in lsp types.
// The magic +4s are to skip the "def" keyword.
impl Range {
	pub fn from_identifier(name: &String, location: ast::Location) -> Self {
		let end_location = ast::Location::new(location.row(), location.column() + name.len());
		Range {
			start: location,
			end: end_location,
		}
	}

	pub fn as_lsp_range(&self) -> LspRange {
		LspRange::new(
			LspPosition::new(self.start.row() as u64 - 1, self.start.column() as u64 - 1 + 4),
			LspPosition::new(self.end.row() as u64 - 1, self.end.column() as u64 - 1 + 4),
		)
	}

	pub fn contains_position(&self, position: &tower_lsp::lsp_types::Position) -> bool {
		self.start.row() - 1 <= position.line as usize
			&& self.start.column() - 1 <= position.character as usize
			&& self.end.row() - 1 >= position.line as usize
			&& self.end.column() - 1 >= position.character as usize
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

	fn contains_position(&self, position: &tower_lsp::lsp_types::Position) -> bool {
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
		docs.get(doc).map(|index| index.clone())
	}
}

#[derive(Default, Debug, Clone)]
pub struct DocumentIndex {
	declarations: HashMap<String, CallableSymbol>,
	calls: Vec<FunctionCall>,
}

fn process_statement(index: &mut DocumentIndex, statement: &ast::Statement) {
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef { name, body, .. } => {
			index.declarations.insert(
				name.clone(),
				CallableSymbol::declared_in_file(name, location),
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
	pub fn call_at(&self, position: &tower_lsp::lsp_types::Position) -> Option<FunctionCall> {
		self.calls
			.iter()
			.find(|call| call.contains_position(position))
			.cloned()
	}

	pub fn declaration_of(&self, call: &FunctionCall) -> Option<CallableSymbol> {
		self.declarations.get(&call.function_name).cloned()
	}
}
