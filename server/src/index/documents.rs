use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use crate::ast::process_document;
use tower_lsp::lsp_types as lsp;

use crate::index::indexed_document::IndexedDocument;
use crate::index::function_decl::{FunctionDecl, CallableSymbolSource};
use crate::bazel::Bazel;

#[derive(Default, Debug)]
pub struct Documents {
	// TODO This really wants to be its own type,
	// so that we don't need to pass maps around in index_document_inner
	docs: RwLock<HashMap<PathBuf, Arc<IndexedDocument>>>,
}

impl Documents {
	pub fn refresh_doc(&self, doc: &PathBuf, bazel: &Bazel) {
		self.index_document(doc, bazel).expect("Trouble refreshing doc");
	}

	pub fn get_doc(&self, doc: &PathBuf) -> Option<Arc<IndexedDocument>> {
		let docs = &*self.docs.read().expect("Failed to lock");
		docs.get(doc).cloned()
	}

	fn index_document(&self, path: &PathBuf, bazel: &Bazel) -> Result<(), String> {
		let index = &mut *self
			.docs
			.write()
			.map_err(|err| format!("Failed to lock documents: {:?}", err))?;
		Documents::index_document_inner(index, path, bazel)
	}

	fn index_document_inner(
		index: &mut HashMap<PathBuf, Arc<IndexedDocument>>,
		path: &PathBuf,
		bazel: &Bazel,
	) -> Result<(), String> {
		let (indexed_doc, docs_to_load) = process_document(path, bazel)?;
		index.insert(path.clone(), Arc::new(indexed_doc));
		for doc in docs_to_load {
			// We unconditionally update the current document,
			// but not any other one, because they haven't changed.
			//
			// If they had, we'd have updated them on did_change.
			if !index.contains_key(&doc) {
				Documents::index_document_inner(index, &doc, bazel)?;
			}
		}
		Ok(())
	}

	pub fn locate_declaration_of_call_at(
		&self,
		doc: &PathBuf,
		position: lsp::Position,
	) -> Option<lsp::Location> {
		let indexed_doc = self.get_doc(doc);
		if let Some(indexed_doc) = indexed_doc {
			let maybe_call = indexed_doc.call_at(position);
			if let Some(call) = maybe_call {
				if let Some(decl) = indexed_doc.declaration_of(&call.function_name) {
					Some(self.locate_declaration(&decl, doc))
				} else {
					None
				}
			} else {
				None
			}
		} else {
			None
		}
	}

	fn locate_declaration(&self, start: &FunctionDecl, current_file: &PathBuf) -> lsp::Location {
		match &start.source {
			CallableSymbolSource::DeclaredInFile(range) => lsp::Location::new(
				lsp::Url::from_file_path(current_file.clone()).expect("Err!"),
				range.as_lsp_range(),
			),
			CallableSymbolSource::Loaded(loaded_path) => {
				let new_declaration = self
					.get_doc(&loaded_path)
					.expect("Error")
					.declaration_of(&start.real_name)
					.unwrap();
				self.locate_declaration(&new_declaration, &loaded_path)
			}
		}
	}
}
