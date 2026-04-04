import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration('skillforge');
  const serverPath = config.get<string>('lspPath', 'aif-lsp');

  const serverOptions: ServerOptions = {
    run: { command: serverPath, args: [] },
    debug: { command: serverPath, args: ['--debug'] },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'aif' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/*.aif'),
    },
  };

  client = new LanguageClient(
    'skillforge-aif',
    'SkillForge AIF',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) return undefined;
  return client.stop();
}
