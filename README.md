# Bazel Lsp

This is a Bazel LSP implementation.
The main goal is to allow to rule-writers and build maintainers quick navigation through external rulesets. In short, I wanted to be able to "goto definition" in starlark files.

## Overall design
After trying many frameworks, I've settled on:
- Using Rust as the implementaiton language. I love Rust, and it has solid support for LSP, as well as a Starlark implementation and several python parsers.
- Using `tower_lsp` as the LSP framework. It's based on `tokio`, and built on top of `lsp_types` and handles all the handshaking and protocol passing. This means that implementing a language server essentially boils down to implementing the `LanguageServer` trait.
- Using a python parser to parse starlark. With that, I build an index of declared functions and function calls, which I can then resolve requests against.
- Uses `bazel sync` to refresh the workspace, and `bazel info execution_root` to figure out where to load the external rulesets from.

- Uses a stripped-down version of the vscode language plugin example [https://github.com/microsoft/vscode-extension-samples/tree/master/lsp-sample]() as the base for VSCode integration, with [https://github.com/bazelbuild/vscode-bazel]()'s syntax files.

## Current Features

- [X] Goto definition in functions declared in the same file.
- [X] Goto definition to files loaded from the workspace.
- [X] Goto definition to files loaded from a different workspace.
- [X] Support loading local references ("//:") that happen in external deps.
- [ ] Add tests, at least integration.
- [ ] Autocomplete.
- [X] Goto definition of symbols that are not functions. 
  - [ ] TODO Note that we will override outer symbols with inner symbols that have hte same identifier. We should only consider the root scope.
- [X] Parse loaded files at parse time
- [X] Model loaded symbols as links to declarations instead of declarations with links. Then put the link-following logic in the document map.
- [X] Run `bazel sync` with custom output base on workspace refreshes.
- [ ] Run `bazel sync` on changes to `WORKSPACE`.
- [ ] Proper error handling, no more expects.
- [ ] Either integrate with a real vscode plugin, or implement proper UX (e.g. progress bars for syncing).

## Disclaimer
As you might have guessed, this project is still in increadibly early stages, I'm building it in my free time. No guarantees given.

This also means that I'm not really looking for contributions right now, as I probably won't have the time to review them. However, all sorts of other feedback is appreciated!