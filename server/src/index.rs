use rustpython_parser::ast::*;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::parser::parse;

#[derive(Debug, Clone)]
pub enum CallableSymbolSource {
	Stdlib,
	DeclaredInFile(Location),
	Loaded(String, PathBuf),
}

#[derive(Debug, Clone)]
pub struct CallableSymbol {
	imported_name: String,
	real_name: String,
	source: CallableSymbolSource,
}

impl CallableSymbol {
	fn declared_in_file(name: &String, location: Location) -> Self {
		CallableSymbol {
			imported_name: name.clone(),
			real_name: name.clone(),
			source: CallableSymbolSource::DeclaredInFile(location),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Range {
	start: Location,
	end: Location,
}

impl Range {
	pub fn contains_position(&self, position: &tower_lsp::lsp_types::Position) -> bool {
		self.start.row() <= position.line as usize &&
		self.start.column() <= position.character as usize &&
		self.end.row() >= position.line as usize &&
		self.end.column() >= position.character as usize
	}
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
	range:  Range,
	function_name: String,
}

impl FunctionCall {
	fn from_identifier(name: &String, location: Location ) -> Self {
		let end_location = Location::new(
			location.row(),
			location.column() + name.len(),
		);
		FunctionCall {
			range: Range { start: location, end: end_location },
			function_name: name.clone(),
		}
	}

	fn contains_position(&self, position: &tower_lsp::lsp_types::Position) -> bool {
		self.range.contains_position(position)
	}
}

#[derive(Default, Debug)]
pub struct Documents {
	docs: Mutex<HashMap<PathBuf, DocumentIndex>>
}

impl Documents {
	pub fn refresh_doc(&self, doc: &PathBuf) {
		let index = &mut *self.docs.lock().expect("");
		let index_for_doc =	DocumentIndex::index_document(doc).expect("");
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
	declarations: Vec<CallableSymbol>,
	calls: Vec<FunctionCall>,
}

fn process_statement(index: &mut DocumentIndex, statement: &Statement) {
	let location = statement.location;
	match &statement.node {
		StatementType::FunctionDef { name, body, .. } => {
			index.declarations.push(CallableSymbol::declared_in_file(name, location));
			process_suite(index, body);
		},
		StatementType::Expression { expression } => match &expression.node {
			ExpressionType::Call { function, .. } => match &function.node {
				ExpressionType::Identifier { name, .. } => index.calls.push(FunctionCall::from_identifier(name, function.location)),
				_ => {},
			},
			_ => {},
		},
		_ => {},
	};
}

fn process_suite(index: &mut DocumentIndex, suite: &Suite) {
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
		self.calls.iter().find(|call| call.contains_position(position)).map(|call| call.clone())
	}
}
