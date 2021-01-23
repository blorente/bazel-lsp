# Bazel Lsp

This is a Bazel LSP implementation.
The main goal is to allow to rule-writers and build maintainers quick navigation through external rulesets. In short, I wanted to be able to "goto definition" in starlark files.

## Overall design
After trying many frameworks, I've settled on:
- Using Rust as the implementaiton language. I love Rust, and it has solid support for LSP, as well as a Starlark implementation and several python parsers.
- Using `tower_lsp` as the LSP framework. It's based on `tokio`, and built on top of `lsp_types` and handles all the handshaking and protocol passing. This means that implementing a language server essentially boils down to implementing the `LanguageServer` trait.
- Using a python parser to parse starlark. With that, I build an index of declared functions and function calls, which I can then resolve requests against. There is still the questions about how I'm going to get the paths to the downloaded external rulesets, but I can use Bazel to resolve that.

## Current Features

- [X] Goto definition in functions declared in the same file.
- [ ] Goto definition to files loaded from the workspace.
- [ ] Goto definition to files loaded from a different workspace.
- [ ] Autocomplete.
- [ ] Goto definition of symbols that are not functions.

## Disclaimer
As you might have guessed, this project is still in increadibly early stages, I'm building it in my free time. No guarantees given.
