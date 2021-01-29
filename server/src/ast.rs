use rustpython_parser::ast;
use rustpython_parser::parser;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;

use crate::indexed_document::IndexedDocument;
use crate::function_decl::FunctionDecl;
use crate::function_call::FunctionCall;

pub fn parse(file: &PathBuf) -> Result<ast::Program, String> {
	let content: String = read_to_string(file).map_err(|err| format!("Error reading file to string: {:?}", err))?;
	parser::parse_program(&content).map_err(|err| format!("Error parsing program: {:?}", err))
}

pub fn process_document(documents: &mut HashMap<PathBuf, Arc<IndexedDocument>>, path: &PathBuf) -> Result<(), String> {
	let ast = parse(path)?;
	let mut index = IndexedDocument::new(path);
	process_suite(&mut index, &ast.statements)?;
	documents.insert(path.clone(), Arc::new(index));
	Ok(())
}

fn process_suite(index: &mut IndexedDocument, suite: &ast::Suite) -> Result<Vec<PathBuf>, String> {
	let mut documents_left_to_parse = vec![];
	for stmt in suite.iter() {
		let docs_to_parse_in_stmt = process_statement(index, stmt)?;
		documents_left_to_parse.extend(docs_to_parse_in_stmt);
	}
	Ok(documents_left_to_parse)
}


fn process_statement(index: &mut IndexedDocument, statement: &ast::Statement) -> Result<Vec<PathBuf>, String> {
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef { name, body, .. } => {
			index
				.declarations
				.insert(name.clone(), FunctionDecl::declared_in_file(name, location));
			Ok(process_suite(index, body)?)
		}
		ast::StatementType::Expression { expression } => match &expression.node {
			ast::ExpressionType::Call {
				function,
				args,
				keywords,
			} => match &function.node {
				ast::ExpressionType::Identifier { name, .. } => match name {
					name if name == "load" => {
						let (loaded_declarations, files_to_load) = process_load(&args, &keywords)?;
						index.declarations.extend(loaded_declarations);
						Ok(files_to_load)
					}
					_ => {
						index
						.calls
						.push(FunctionCall::from_identifier(name, function.location));
						Ok(vec![])
					},
				},
				_ => {Ok(vec![])}
			},
			_ => {Ok(vec![])}
		},
		_ => {Ok(vec![])}
	}
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
	args: &Vec<ast::Expression>,
	kwargs: &Vec<ast::Keyword>,
) -> Result<(HashMap<String, FunctionDecl>, Vec<PathBuf>), String> {
	let mut declarations = HashMap::new();
	let source = process_string_literal(&args[0]);
	for arg in &args[1..args.len()] {
		let name = process_string_literal(&arg);
		declarations.insert(name.clone(), FunctionDecl::loaded(&name, &name, &source));
	}
	for kwarg in kwargs {
		let imported_name = kwarg.name.as_ref().cloned().ok_or_else(|| "Kwarg without a name")?;
		let real_name = process_string_literal(&kwarg.value);
		declarations.insert(
			imported_name.clone(),
			FunctionDecl::loaded(&real_name, &imported_name, &source),
		);
	}
	Ok((declarations, vec![]))
}
