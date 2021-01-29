use rustpython_parser::ast;
use rustpython_parser::parser;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tower_lsp::lsp_types as lsp;

pub fn parse(file: &PathBuf) -> Result<ast::Program, ()> {
	let content: String = read_to_string(file).map_err(|_| ())?;
	parser::parse_program(&content).map_err(|_| ())
}

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

fn resolve_bazel_path(path: &String) -> PathBuf {
	if path.starts_with("//:") {
		let resolved_path = std::env::current_dir()
					.expect("Error getting current dir.")
					.as_os_str()
					.to_str()
					.expect("Error converting current dir to string")
					.to_owned() + "/" + path.strip_prefix("//:").unwrap();
		
		PathBuf::from(resolved_path)
	} else {
		panic!("Path {} didn't start with //:", path);
	}
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

	fn loaded(name: &String, imported_name: &String, source: &String) -> Self {
		FunctionDecl {
			imported_name: imported_name.clone(),
			real_name: name.clone(),
			source: CallableSymbolSource::Loaded(resolve_bazel_path(source)),
		}
	}

	pub fn lsp_location(&self, current_file: &tower_lsp::lsp_types::Url) -> lsp::Location {
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
	docs: Mutex<HashMap<PathBuf, Arc<IndexedDocument>>>,
}

impl Documents {
	pub fn refresh_doc(&self, doc: &PathBuf) {
		self.index_document(doc).expect("Trouble refreshing doc");
	}

	pub fn get_doc(&self, doc: &PathBuf) -> Option<Arc<IndexedDocument>> {
		let docs = &*self.docs.lock().expect("Failed to lock");
		// TODO This clone could get very expensive, we should wrap indexes in Arcs
		docs.get(doc).cloned()
	}

	pub fn index_document(&self, path: &PathBuf) -> Result<(), ()> {
		let index = &mut *self.docs.lock().map_err(|_| ())?;
		process_document(index, path)?;
		Ok(())
	}

}
fn process_suite(index: &mut IndexedDocument, suite: &ast::Suite) {
	for stmt in suite {
		process_statement(index, &stmt);
	}
}

fn process_document(documents: &mut HashMap<PathBuf, Arc<IndexedDocument>>, path: &PathBuf) -> Result<(), ()> {
	let ast = parse(path)?;
	let mut index = IndexedDocument::new(path);
	process_suite(&mut index, &ast.statements);
	documents.insert(path.clone(), Arc::new(index));
	Ok(())
}


fn process_statement(index: &mut IndexedDocument, statement: &ast::Statement) {
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef { name, body, .. } => {
			index
				.declarations
				.insert(name.clone(), FunctionDecl::declared_in_file(name, location));
			process_suite(index, body);
		}
		ast::StatementType::Expression { expression } => match &expression.node {
			ast::ExpressionType::Call {
				function,
				args,
				keywords,
			} => match &function.node {
				ast::ExpressionType::Identifier { name, .. } => match name {
					name if name == "load" => process_load(index, &args, &keywords),
					_ => index
						.calls
						.push(FunctionCall::from_identifier(name, function.location)),
				},
				_ => {}
			},
			_ => {}
		},
		_ => {}
	};
}

fn process_string_literal(expr: &ast::Expression) -> String {
	if let ast::ExpressionType::String { value } = &expr.node {
		if let ast::StringGroup::Constant { value } = value {
			value.clone()
		} else {
			panic!("Loaded symbol with non-constant {:?}", expr)
		}
	} else {
		panic!("Couldn't understand loaded symbol {:?}", expr)
	}
}

fn process_load(
	index: &mut IndexedDocument,
	args: &Vec<ast::Expression>,
	kwargs: &Vec<ast::Keyword>,
) {
	let source = process_string_literal(&args[0]);
	for arg in &args[1..args.len()] {
		let name = process_string_literal(&arg);
		index
			.declarations
			.insert(name.clone(), FunctionDecl::loaded(&name, &name, &source));
	}
	for kwarg in kwargs {
		let imported_name = kwarg.name.as_ref().expect("Kwarg without a name").clone();
		let real_name = process_string_literal(&kwarg.value);
		index.declarations.insert(
			imported_name.clone(),
			FunctionDecl::loaded(&real_name, &imported_name, &source),
		);
	}
}

#[derive(Default, Debug, Clone)]
pub struct IndexedDocument {
	declarations: HashMap<String, FunctionDecl>,
	calls: Vec<FunctionCall>,
	path: PathBuf,
}

impl IndexedDocument {
	fn new(path: &PathBuf) -> Self {
		IndexedDocument {
			path: path.clone(),
			declarations: HashMap::default(),
			calls: Vec::default(),
		}
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
