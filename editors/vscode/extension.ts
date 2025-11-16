import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

/**
 * Called by VS Code when the extension is activated.
 */
export function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration('shrimpl');
  const configuredPath = config.get<string>('lsp.path');

  // Default: look for `shrimpl-lsp` in PATH
  const command = configuredPath && configuredPath.trim().length > 0
    ? configuredPath.trim()
    : 'shrimpl-lsp';

  const serverOptions: ServerOptions = {
    command,
    args: []
  };

  const clientOptions: LanguageClientOptions = {
    // LSP will be attached to .shr files
    documentSelector: [{ scheme: 'file', language: 'shrimpl' }],
    synchronize: {
      // Watch Shrimpl files for changes
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.shr')
    }
  };

  client = new LanguageClient(
    'shrimplLanguageServer',
    'Shrimpl Language Server',
    serverOptions,
    clientOptions
  );

  context.subscriptions.push(client.start());
}

/**
 * Called by VS Code when the extension is deactivated.
 */
export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
