use rustpython_parser::ast::*;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::parser::parse;

#[derive(Debug)]
pub enum CallableSymbolSource {
	Stdlib,
	DeclaredInFile(Location),
	Loaded(String, PathBuf),
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct FunctionCall {
	range: (Location, Location),
	function_name: String,
}

impl FunctionCall {
	fn from_identifier(name: &String, location: Location ) -> Self {
		let end_location = Location::new(
			location.row(),
			location.column() + name.len(),
		);
		FunctionCall {
			range: (location, end_location),
			function_name: name.clone(),
		}
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
}

#[derive(Default, Debug)]
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
}
