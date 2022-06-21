/// A Symbol contains all the information needed to uniquely identify a Starlark symbol.
///
/// For instance, when we `load("@my_repo//my/file:file.bzl", "symbol")`,
/// The root is `"@my_repo//my/file:file.bzl"`, while the name is `"symbol"`.
/// The fully qualified name of a symbol is `{root}#{name}`, which sould be sufficient to be a
/// unique identifier across a single workspace.

pub(crate) type Root = String; // TODO make this separate the repo from the path.
pub(crate) type Name = String;
pub(crate) struct Symbol(Root, Name);

impl Symbol {
    pub fn fully_qualified_name(&self) -> String {
        format!("{}#{}", self.0, self.1)
    }
}
