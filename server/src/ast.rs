use rustpython_parser::ast;
use rustpython_parser::error;
use rustpython_parser::parser;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use crate::bazel::BazelWorkspace;
use crate::index::function_call::FunctionCall;
use crate::index::function_decl::FunctionDecl;
use crate::index::indexed_document::IndexedDocument;

pub fn process_document(
	contents: &str,
	bazel: &BazelWorkspace,
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
	bazel: &BazelWorkspace,
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
	bazel: &BazelWorkspace,
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
	bazel: &BazelWorkspace,
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
	bazel: &BazelWorkspace,
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
	use crate::bazel::BazelWorkspace;
	use crate::bazel::BazelInfo;

	use trim_margin::MarginTrimmable;
	use rustpython_parser::ast;
    use tempfile;

	use std::io::{self, Write};

	#[derive(Default)]
    struct MockBazelInfo {
		mock_exec_root: PathBuf,
	}
	impl MockBazelInfo {
		fn new(root: &PathBuf) -> Self { MockBazelInfo{mock_exec_root: root.clone() }}
	}
	impl BazelInfo for MockBazelInfo {
		fn get_exec_root(&self, source_root: &PathBuf) -> Result<PathBuf, String> {
           Ok(self.mock_exec_root.clone())           
		}
	    fn debug(&self) -> String {
           format!("MockBazelInfo({:?})", &self.mock_exec_root)
		}
	}

	// Note we return the tempdir here because otherwise it will be deleted when it goes out of scope.
	fn with_workspace<F, T: Sized>(contents: HashMap<&str, &str>, mut body: F) -> T where F: FnMut(PathBuf) -> T {
		let root = tempfile::tempdir().unwrap();
		for (filepath, content) in contents {
            let path = root.path().join(filepath);
			println!("Creating file {:?}", path);
			let mut file = std::fs::File::create(&path).unwrap();
			writeln!(file, "{}", content).unwrap();
			assert!(&path.is_file(), "File is not file!");
		}
		body(PathBuf::from(root.path()))
	}

	fn run_parse(file: &str, files_in_workspace: HashMap<&str, &str>) -> (IndexedDocument, Vec<PathBuf>) {
		with_workspace(files_in_workspace, |repo_root| {
			let bazel = BazelWorkspace::with_bazel_info(Box::new(MockBazelInfo::new(&repo_root)));
			let bazel_res = bazel.update_workspace(&repo_root);
			assert!(bazel_res.is_ok(), "Failed to update bazel repository!:\n {:?}", bazel_res);
			let parse_result = super::process_document(file, &bazel);
			assert!(parse_result.is_ok(), "Failed to parse file:\n {} \n {:?}", file, parse_result);
			parse_result.unwrap()
		})
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
	fn test_load_statement() {
		let file = trimmed("
		|load('//:some_file.bzl', 
		|     'loaded_func',
		|     loaded_and_renamed_func = 'other_func',
	    |)
		|loaded_func()
		|loaded_and_renamed_func()
		");
		//let (indexed_document, paths_to_load) = run_parse(&file, Some(mock_bazel(vec!["some_file.bzl"], vec![])));
		let (indexed_document, paths_to_load) = run_parse(&file, hashmap!{"some_file.bzl" => ""});

		let expected_indexed_document = IndexedDocument::finished(
			hashmap! {
			  "loaded_func".to_string() => declaration_loaded("loaded_func", None, "some_file.bzl"),
			  "loaded_and_renamed_func".to_string() => declaration_loaded("loaded_func", Some("loaded_and_renamed_func"), "some_file.bzl"),
			},
			vec![
				call("loaded_func", location(1, 0)),
			],
		);
		assert_eq!(indexed_document.declarations, expected_indexed_document.declarations);
		assert_eq!(indexed_document.calls, expected_indexed_document.calls);
		assert!(paths_to_load.is_empty());
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
