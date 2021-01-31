use rustpython_parser::ast;
use rustpython_parser::parser;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

use crate::bazel::Bazel;
use crate::index::function_call::FunctionCall;
use crate::index::function_decl::FunctionDecl;
use crate::index::indexed_document::IndexedDocument;

pub fn parse(file: &PathBuf) -> Result<ast::Program, String> {
	let content: String = read_to_string(file)
		.map_err(|err| format!("Error reading file {:?} to string: {:?}", &file, err))?;
	parser::parse_program(&content)
		.map_err(|err| format!("Error parsing file {:?}: {:?}", &file, err))
}

pub fn process_document(
	path: &PathBuf,
	bazel: &Bazel,
) -> Result<(IndexedDocument, Vec<PathBuf>), String> {
	let ast = parse(path)?;
	let mut indexed_document = IndexedDocument::new(path);
	let docs_to_load = process_suite(&mut indexed_document, &ast.statements, bazel)?;
	Ok((indexed_document, docs_to_load))
}

fn process_suite(
	index: &mut IndexedDocument,
	suite: &ast::Suite,
	bazel: &Bazel,
) -> Result<Vec<PathBuf>, String> {
	let mut documents_left_to_parse = vec![];
	for stmt in suite.iter() {
		let docs_to_parse_in_stmt = process_statement(index, stmt, bazel)?;
		documents_left_to_parse.extend(docs_to_parse_in_stmt);
	}
	Ok(documents_left_to_parse)
}

fn process_statement(
	index: &mut IndexedDocument,
	statement: &ast::Statement,
	bazel: &Bazel,
) -> Result<Vec<PathBuf>, String> {
	let location = statement.location;
	match &statement.node {
		ast::StatementType::FunctionDef { name, body, .. } => {
			let location_with_def =
				ast::Location::new(location.row(), location.column() + "def ".len());
			index.declarations.insert(
				name.clone(),
				FunctionDecl::declared_in_file(name, location_with_def),
			);
			Ok(process_suite(index, body, bazel)?)
		}
		ast::StatementType::Assign { targets, value } => {
			for target in targets {
				match &target.node {
					ast::ExpressionType::Identifier { name, .. } => {
						index.declarations.insert(
							name.clone(),
							FunctionDecl::declared_in_file(name, target.location),
						);
					}
					_ => {}
				};
			};
			process_rhs_expression(value, index, bazel)
		}
		ast::StatementType::Expression { expression } => {
			process_rhs_expression(&expression, index, bazel)
		}
		_ => Ok(vec![]),
	}
}

fn process_rhs_expression(
	expression: &ast::Expression,
	index: &mut IndexedDocument,
	bazel: &Bazel,
) -> Result<Vec<PathBuf>, String> {
	match &expression.node {
		ast::ExpressionType::Identifier {
			name, ..
		} => {
			index.calls.push(FunctionCall::from_identifier(&name, expression.location));
			Ok(vec![])
		},
		ast::ExpressionType::Call {
			function,
			args,
			keywords,
		} => match &function.node {
			ast::ExpressionType::Identifier { name, .. } => match name {
				name if name == "load" => process_load(&args, &keywords, index, bazel),
				_ => {
					// None of these should have files to load, as they are not top-level loads.
					process_rhs_expression(&function, index, bazel)?;
					for arg in args {
						process_rhs_expression(&arg, index, bazel)?;
					}
					for kwarg in keywords {
						process_rhs_expression(&kwarg.value, index, bazel)?;
					}
					Ok(vec![])
				}
			},
			_ => Ok(vec![]),
		},
		_ => Ok(vec![]),
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
	index: &mut IndexedDocument,
	bazel: &Bazel,
) -> Result<Vec<PathBuf>, String> {
	let source = process_string_literal(&args[0]);
	let maybe_source_as_path = bazel.resolve_bazel_path(&source);
	if let Ok(source_as_path) = maybe_source_as_path {
		let mut declarations = HashMap::new();
		for arg in &args[1..args.len()] {
			let name = process_string_literal(&arg);
			declarations.insert(
				name.clone(),
				FunctionDecl::loaded(&name, &name, &source_as_path),
			);
		}
		for kwarg in kwargs {
			let imported_name = kwarg
				.name
				.as_ref()
				.cloned()
				.ok_or_else(|| "Kwarg without a name")?;
			let real_name = process_string_literal(&kwarg.value);
			declarations.insert(
				imported_name.clone(),
				FunctionDecl::loaded(&real_name, &imported_name, &source_as_path),
			);
		}
		index.declarations.extend(declarations);
		Ok(vec![source_as_path])
	} else {
		// For now, we don't want to error when we find a file we cannot load.
		// However, given that we're not going to be able to goto definiton,
		// no sense in returning any declarations.
		Ok(vec![])
	}
}
