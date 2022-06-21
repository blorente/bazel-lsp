use std::collections::HashMap;

/// A Symbol contains all the information needed to uniquely identify a Starlark symbol.
///
/// For instance, when we `load("@my_repo//my/file:file.bzl", "symbol")`,
/// The root is `"@my_repo//my/file:file.bzl"`, while the name is `"symbol"`.
/// The fully qualified name of a symbol is `{root}#{name}`, which sould be sufficient to be a
/// unique identifier across a single workspace.
///
use crate::vfs::FileId;
use eyre::{eyre, Result};

pub(crate) type Root = String; // TODO make this separate the repo from the path.
pub(crate) type Name = String;
#[derive(Hash, std::cmp::PartialEq, std::cmp::Eq, Debug)]
pub(crate) struct Symbol(Root, Name);

impl Symbol {
    pub(crate) fn from_fqn(fqn: &str) -> Result<Self> {
        Err(eyre!("Symbol::from_fqn is unimplemented!"))
    }

    pub(crate) fn fully_qualified_name(&self) -> String {
        format!("{}#{}", self.0, self.1)
    }
}

#[derive(Debug)]
pub(crate) struct SymbolLocation {
    file: FileId,
    line: usize,
    column: usize,
}

#[derive(Debug)]
pub(crate) struct SymbolIndex {
    db: HashMap<Symbol, SymbolLocation>,
}

impl SymbolIndex {
    pub(crate) fn new() -> Self {
        SymbolIndex { db: HashMap::new() }
    }

    pub(crate) fn register_symbol(
        &mut self,
        fully_qualified_name: String,
        file: FileId,
        line: usize,
        column: usize,
    ) -> Result<()> {
        let symbol = Symbol::from_fqn(&fully_qualified_name)?;
        let loc = SymbolLocation { file, line, column };
        self.db.insert(symbol, loc);
        Ok(())
    }
}
