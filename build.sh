#!/bin/bash
set -e

# Compile server
(cd rewrite && cargo build)

# Compile extension
(cd lsp-vscode-extension && npm run compile)
