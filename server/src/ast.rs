use rustpython_parser::ast;
use rustpython_parser::error;
use rustpython_parser::parser;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use crate::bazel::BazelResolver;
use crate::index::function_call::FunctionCall;
use crate::index::function_decl::FunctionDecl;
use crate::index::indexed_document::IndexedDocument;

pub fn process_document(
	contents: &str,
	bazel: &dyn BazelResolver,
) -> Result<(IndexedDocument, Vec<PathBuf>), String> {
	let ast = parser::parse_program(contents)
		.map_err(|err| format!("Failed to parse program: {:?}", err))?;
	let mut indexed_document = IndexedDocument::new();
	let docs_to_load = process_suite(&mut indexed_document, &ast.statements, bazel)?;
	Ok((indexed_document, docs_to_load))
}

fn process_suite(
	index: &mut IndexedDocument,
	suite: &ast::Suite,
	bazel: &dyn BazelResolver,
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
	bazel: &dyn BazelResolver,
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
			}
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
	bazel: &dyn BazelResolver,
) -> Result<Vec<PathBuf>, String> {
	match &expression.node {
		ast::ExpressionType::Identifier { name, .. } => {
			index
				.calls
				.push(FunctionCall::from_identifier(&name, expression.location));
			Ok(vec![])
		}
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
	bazel: &dyn BazelResolver,
) -> Result<Vec<PathBuf>, String> {
	let source = process_string_literal(&args[0]);
	let maybe_source_as_path = bazel.resolve_bazel_path(&source);
	match maybe_source_as_path {
		Ok(source_as_path) => {
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
		}
		Err(s) => Err(s)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::bazel::BazelResolver;

	use trim_margin::MarginTrimmable;
	use rustpython_parser::ast;
    use tempfile;

	use std::io::{self, Write};

    struct MockBazelResolver {
		files_in_workspace: HashMap<String, String>
	}
	impl MockBazelResolver {
        pub fn new(files_in_workspace: HashMap<&str, &str>) -> Self {
			MockBazelResolver {
				files_in_workspace: files_in_workspace.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>()
			}
		}
	}
	impl BazelResolver for MockBazelResolver {
	    fn resolve_bazel_path(&self, path: &String) -> Result<PathBuf, String> {
			let sanitized_path = &self.sanitize_starlark_label(path);
			if self.files_in_workspace.contains_key(sanitized_path) {
		        Ok(PathBuf::from(sanitized_path))
			} else {
				Err(format!("Path {} not found in repo:\n {:#?}", path, &self.files_in_workspace))
			}
	    }
	}

	fn run_parse(file: &str, files_in_workspace: HashMap<&str, &str>) -> (IndexedDocument, Vec<PathBuf>) {
			let bazel_resolver = MockBazelResolver::new(files_in_workspace);
			let parse_result = super::process_document(file, &bazel_resolver);
			assert!(parse_result.is_ok(), "Failed to parse file:\n {} \n {:?}", file, parse_result);
			parse_result.unwrap()
	}

	fn trimmed(s: &str) -> String {
	    let res = s.trim_margin();
		assert!(res.is_some(), "Failed to trim margin from: \n {}", s);
		res.unwrap()
	}

	fn location(line: usize, col: usize) -> ast::Location {
		ast::Location::new(line + 1, col + 1)
	}

	fn declaration_in_file(name: &str, location: ast::Location) -> FunctionDecl	{
		FunctionDecl::declared_in_file(&name.to_string(), location)
	}

	fn declaration_loaded(name: &str, imported_name: Option<&str>, path: &str) -> FunctionDecl	{
		FunctionDecl::loaded(&name.to_string(), &imported_name.unwrap_or(name).to_string(), &path.into())
	}

	fn call(name: &str, location: ast::Location) -> FunctionCall {
		FunctionCall::from_identifier(&name.to_string(), location)
	}

	#[test]
	fn test_single_assignment() {
		let file = "a = 3";
		let (indexed_document, paths_to_load) = run_parse(&file, hashmap!{});

		let expected_indexed_document = IndexedDocument::finished(
			hashmap! {
			  "a".to_string() => declaration_in_file("a", location(0, 0))
			},
			vec![],
		);
		assert_eq!(indexed_document, expected_indexed_document);
		assert!(paths_to_load.is_empty());
	}

	#[test]
	fn test_single_function_declaration() {
		let file = trimmed("
		|def hello():
		|  call_to_other_function()
		|hello()
		");
		let (indexed_document, paths_to_load) = run_parse(&file, hashmap!{});

		let expected_indexed_document = IndexedDocument::finished(
			hashmap! {
			    "hello".to_string() => declaration_in_file("hello", location(0, 4))
			},
			vec![
				call("call_to_other_function", location(1, 2)),
				call("hello", location(2, 0)),
			],
		);
		assert_eq!(indexed_document.declarations, expected_indexed_document.declarations);
		assert_eq!(indexed_document.calls, expected_indexed_document.calls);
		assert!(paths_to_load.is_empty());
	}

	#[test]
	fn test_load_statement() {
		let file = trimmed("
		|load('//:some_file.bzl', 
		|     'loaded_func',
		|     loaded_and_renamed_func = 'other_func',
	    |)
		|loaded_func()
		|loaded_and_renamed_func(3, 4)
		");
		let (indexed_document, paths_to_load) = run_parse(&file, hashmap!{"some_file.bzl" => ""});

		let expected_indexed_document = IndexedDocument::finished(
			hashmap! {
			  "loaded_func".to_string() => declaration_loaded("loaded_func", None, "some_file.bzl"),
			  "loaded_and_renamed_func".to_string() => declaration_loaded("other_func", Some("loaded_and_renamed_func"), "some_file.bzl"),
			},
			vec![
				call("loaded_func", location(4, 0)),
				call("loaded_and_renamed_func", location(5, 0)),
			],
		);
		assert_eq!(indexed_document.declarations, expected_indexed_document.declarations);
		assert_eq!(indexed_document.calls, expected_indexed_document.calls);
		let expected_paths_to_load = vec![PathBuf::from("some_file.bzl")];
		assert_eq!(paths_to_load, expected_paths_to_load);
	}

	#[test]
	fn test_call_loaded_function_from_declared_function() {
		let file = trimmed("
		|load('//:some_file.bzl', 'loaded_func')
	    |def defined_func():
		|  loaded_func()
		|defined_func()
		");

		let (indexed_document, paths_to_load) = run_parse(&file, hashmap!{"some_file.bzl" => ""});

		let expected_indexed_document = IndexedDocument::finished(
			hashmap! {
			  "loaded_func".to_string() => declaration_loaded("loaded_func", None, "some_file.bzl"),
			  "defined_func".to_string() => declaration_in_file("defined_func", location(1, 4)),
			},
			vec![
				call("loaded_func", location(2, 2)),
				call("defined_func", location(3, 0)),
			],
		);
		assert_eq!(indexed_document.declarations, expected_indexed_document.declarations);
		assert_eq!(indexed_document.calls, expected_indexed_document.calls);
		let expected_paths_to_load = vec![PathBuf::from("some_file.bzl")];
		assert_eq!(paths_to_load, expected_paths_to_load);
	}
	
	// This test fails for now because we don't correctly parse symbols inside functions, such as `a` and `b`.
	#[test] #[ignore]
	fn test_function_declaration() {
		let file = trimmed("
        |def func(a, b):
	    |  a + b
        |func(c(), d())
		");
		let (indexed_document, paths_to_load) = run_parse(&file, hashmap!{});

		let expected_indexed_document = IndexedDocument::finished(
			hashmap! {
			  "func".to_string() => declaration_in_file("func", location(0, 4))
			},
			vec![
				call("func", location(2, 0)),
				call("c", location(2, 5)),
				call("d", location(2, 10)),
				call("a", location(1, 2)),
				call("b", location(1, 6)),
			],
		);
		assert_eq!(indexed_document.declarations, expected_indexed_document.declarations);
		assert_eq!(indexed_document.calls, expected_indexed_document.calls);
		assert!(paths_to_load.is_empty());
	}
}
