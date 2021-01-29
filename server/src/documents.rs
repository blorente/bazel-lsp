use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::indexed_document::IndexedDocument;
use crate::ast::process_document;


#[derive(Default, Debug)]
pub struct Documents {
	// TODO This really wants to be its own type,
	// so that we don't need to pass maps around in index_document_inner
	docs: Mutex<HashMap<PathBuf, Arc<IndexedDocument>>>,
}

impl Documents {
	pub fn refresh_doc(&self, doc: &PathBuf) {
		self.index_document(doc).expect("Trouble refreshing doc");
	}

	pub fn get_doc(&self, doc: &PathBuf) -> Option<Arc<IndexedDocument>> {
		let docs = &*self.docs.lock().expect("Failed to lock");
		docs.get(doc).cloned()
	}

	pub fn index_document(&self, path: &PathBuf) -> Result<(), String> {
		let index = &mut *self.docs.lock().map_err(|err| format!("Failed to lock documents: {:?}", err))?;
		Documents::index_document_inner(index, path)
	}

	fn index_document_inner(index: &mut HashMap<PathBuf, Arc<IndexedDocument>>, path: &PathBuf) -> Result<(), String> {
		let (indexed_doc, docs_to_load) = process_document(path)?;
		index.insert(path.clone(), Arc::new(indexed_doc));
		for doc in docs_to_load {
			Documents::index_document_inner(index, &doc)?;
		}
		Ok(())
	}

}