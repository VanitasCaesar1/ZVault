import * as vscode from 'vscode';

const DEFAULT_URL = 'https://api.zvault.cloud';

interface SecretKey {
  key: string;
  version: number;
  comment: string;
  updated_at: string;
}

let secretKeys: SecretKey[] = [];
let statusBarItem: vscode.StatusBarItem;

export function activate(context: vscode.ExtensionContext) {
  // Status bar
  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBarItem.command = 'zvault.switchEnv';
  updateStatusBar();
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  // Commands
  context.subscriptions.push(
    vscode.commands.registerCommand('zvault.listSecrets', listSecrets),
    vscode.commands.registerCommand('zvault.peekSecret', peekSecret),
    vscode.commands.registerCommand('zvault.refreshSecrets', refreshSecrets),
    vscode.commands.registerCommand('zvault.switchEnv', switchEnv),
  );

  // Autocomplete for env var patterns
  const completionProvider = vscode.languages.registerCompletionItemProvider(
    ['typescript', 'javascript', 'python', 'go', 'rust', 'yaml', 'json', 'dotenv'],
    {
      async provideCompletionItems(document, position) {
        const lineText = document.lineAt(position).text;
        const beforeCursor = lineText.substring(0, position.character);

        // Match patterns like process.env., os.environ[", env::var("
        const envPatterns = [
          /process\.env\.$/,
          /process\.env\["$/,
          /process\.env\['$/,
          /os\.environ\["$/,
          /os\.environ\.get\("$/,
          /env::var\("$/,
          /os\.Getenv\("$/,
          /ZVAULT_/,
        ];

        const isEnvAccess = envPatterns.some((p) => p.test(beforeCursor));
        if (!isEnvAccess && secretKeys.length === 0) return [];

        await ensureSecrets();

        return secretKeys.map((sk) => {
          const item = new vscode.CompletionItem(sk.key, vscode.CompletionItemKind.Variable);
          item.detail = `ZVault secret (v${sk.version})`;
          item.documentation = new vscode.MarkdownString(
            `**${sk.key}**\n\nVersion: ${sk.version}\n\n${sk.comment || 'No description'}\n\nUpdated: ${sk.updated_at}`,
          );
          return item;
        });
      },
    },
    '.', '"', "'", '(',
  );
  context.subscriptions.push(completionProvider);

  // Hover provider — shows secret metadata (not value)
  const hoverProvider = vscode.languages.registerHoverProvider(
    ['typescript', 'javascript', 'python', 'go', 'rust', 'yaml', 'dotenv'],
    {
      async provideHover(document, position) {
        const range = document.getWordRangeAtPosition(position, /[A-Z_][A-Z0-9_]*/);
        if (!range) return;

        const word = document.getText(range);
        await ensureSecrets();

        const secret = secretKeys.find((s) => s.key === word);
        if (!secret) return;

        const env = getConfig('env') || 'development';
        const md = new vscode.MarkdownString();
        md.appendMarkdown(`**🔐 ZVault Secret**: \`${secret.key}\`\n\n`);
        md.appendMarkdown(`- **Environment**: ${env}\n`);
        md.appendMarkdown(`- **Version**: ${secret.version}\n`);
        md.appendMarkdown(`- **Updated**: ${secret.updated_at}\n`);
        if (secret.comment) {
          md.appendMarkdown(`- **Description**: ${secret.comment}\n`);
        }
        md.appendMarkdown(`\n[Peek Value](command:zvault.peekSecret)`);
        md.isTrusted = true;

        return new vscode.Hover(md, range);
      },
    },
  );
  context.subscriptions.push(hoverProvider);

  // Initial fetch
  refreshSecrets();
}

export function deactivate() {
  secretKeys = [];
}

function getConfig(key: string): string {
  const config = vscode.workspace.getConfiguration('zvault');
  const val = config.get<string>(key) ?? '';
  // Also check env vars
  if (!val) {
    const envMap: Record<string, string> = {
      token: 'ZVAULT_TOKEN',
      orgId: 'ZVAULT_ORG_ID',
      projectId: 'ZVAULT_PROJECT_ID',
      env: 'ZVAULT_ENV',
      url: 'ZVAULT_URL',
    };
    return process.env[envMap[key] ?? ''] ?? '';
  }
  return val;
}

function updateStatusBar() {
  const env = getConfig('env') || 'development';
  statusBarItem.text = `$(lock) ZVault: ${env}`;
  statusBarItem.tooltip = `ZVault — ${secretKeys.length} secrets loaded`;
}

async function ensureSecrets() {
  if (secretKeys.length === 0) {
    await refreshSecrets();
  }
}

async function refreshSecrets() {
  const token = getConfig('token');
  const orgId = getConfig('orgId');
  const projectId = getConfig('projectId');
  const env = getConfig('env') || 'development';
  const url = (getConfig('url') || DEFAULT_URL).replace(/\/+$/, '');

  if (!token || !orgId || !projectId) {
    statusBarItem.text = '$(lock) ZVault: Not configured';
    return;
  }

  try {
    const keysUrl = `${url}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${env}/secrets`;
    const res = await fetch(keysUrl, {
      headers: {
        Authorization: `Bearer ${token}`,
        'User-Agent': 'zvault-vscode/0.1.0',
      },
    });

    if (!res.ok) {
      vscode.window.showWarningMessage(`ZVault: Failed to fetch secrets (HTTP ${res.status})`);
      return;
    }

    const data = (await res.json()) as { keys: SecretKey[] };
    secretKeys = data.keys;
    updateStatusBar();
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showWarningMessage(`ZVault: ${msg}`);
  }
}

async function listSecrets() {
  await ensureSecrets();

  if (secretKeys.length === 0) {
    vscode.window.showInformationMessage('ZVault: No secrets found.');
    return;
  }

  const items = secretKeys.map((s) => ({
    label: s.key,
    description: `v${s.version}`,
    detail: s.comment || undefined,
  }));

  const selected = await vscode.window.showQuickPick(items, {
    placeHolder: 'Select a secret to copy its key',
  });

  if (selected) {
    await vscode.env.clipboard.writeText(selected.label);
    vscode.window.showInformationMessage(`Copied "${selected.label}" to clipboard`);
  }
}

async function peekSecret() {
  const editor = vscode.window.activeTextEditor;
  if (!editor) return;

  const selection = editor.selection;
  let key = editor.document.getText(selection);

  if (!key) {
    // Try to get word under cursor
    const range = editor.document.getWordRangeAtPosition(selection.active, /[A-Z_][A-Z0-9_]*/);
    if (range) key = editor.document.getText(range);
  }

  if (!key) {
    key =
      (await vscode.window.showInputBox({ prompt: 'Enter secret key to peek' })) ?? '';
  }

  if (!key) return;

  const token = getConfig('token');
  const orgId = getConfig('orgId');
  const projectId = getConfig('projectId');
  const env = getConfig('env') || 'development';
  const url = (getConfig('url') || DEFAULT_URL).replace(/\/+$/, '');

  if (!token || !orgId || !projectId) {
    vscode.window.showWarningMessage('ZVault: Not configured. Set zvault.token, zvault.orgId, zvault.projectId in settings.');
    return;
  }

  try {
    const secretUrl = `${url}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${env}/secrets/${encodeURIComponent(key)}`;
    const res = await fetch(secretUrl, {
      headers: { Authorization: `Bearer ${token}` },
    });

    if (!res.ok) {
      vscode.window.showWarningMessage(`ZVault: Secret "${key}" not found in "${env}"`);
      return;
    }

    const data = (await res.json()) as { secret: { key: string; value: string; version: number } };

    const action = await vscode.window.showInformationMessage(
      `🔐 ${key} (v${data.secret.version}): ${maskValue(data.secret.value)}`,
      'Copy Value',
      'Copy Key',
    );

    if (action === 'Copy Value') {
      await vscode.env.clipboard.writeText(data.secret.value);
      vscode.window.showInformationMessage('Secret value copied to clipboard');
    } else if (action === 'Copy Key') {
      await vscode.env.clipboard.writeText(key);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`ZVault: ${msg}`);
  }
}

async function switchEnv() {
  const envs = ['development', 'staging', 'production', 'test'];
  const selected = await vscode.window.showQuickPick(envs, {
    placeHolder: 'Select environment',
  });

  if (selected) {
    const config = vscode.workspace.getConfiguration('zvault');
    await config.update('env', selected, vscode.ConfigurationTarget.Workspace);
    secretKeys = [];
    await refreshSecrets();
    vscode.window.showInformationMessage(`ZVault: Switched to "${selected}"`);
  }
}

function maskValue(value: string): string {
  if (value.length <= 8) return '••••••••';
  return value.substring(0, 4) + '••••' + value.substring(value.length - 4);
}
