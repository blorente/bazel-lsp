/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
	// The server is implemented in node
	// let serverModule = context.asAbsolutePath(
	// 	path.join('server', 'out', 'server.js')
	// );
    let serverModule = context.asAbsolutePath(path.join('..', 'server', 'target', 'debug', 'server'))
	// The debug options for the server
	// --inspect=6009: runs the server in Node's Inspector mode so VS Code can attach to the server for debugging
	// let debugOptions = { execArgv: ['--nolazy', '--inspect=6009'] };
	let debugOptions = { };

	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used
	let serverOptions: ServerOptions = {command: serverModule};

	// Options to control the language client
	let clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [
			{ scheme: 'file', language: 'starlark' },
			{ scheme: 'file', language: 'plaintext', pattern: '**/WORKSPACE' },
			{ scheme: 'file', language: 'plaintext', pattern: '**/BUILD' },
			{ scheme: 'file', language: 'plaintext', pattern: '**/BUILD.bazel' },
			{ scheme: 'file', language: 'plaintext', pattern: '**/*.bzl' },
			{ scheme: 'file', pattern: '**/tools/build_rules/prelude_bazel' },
		],
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			// TODO Watch bazelrc
			fileEvents: workspace.createFileSystemWatcher('**/WORKSPACE')
		}
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'languageServerExample',
		'Bazel Language Server',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}
