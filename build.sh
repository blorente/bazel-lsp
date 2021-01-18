#!/bin/bash

# Compile server
(cd server && cargo build)

# Compile extension
(cd lsp-vscode-extension && npm run compile)