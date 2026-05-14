import * as vscode from "vscode";

export function createPanel(
  context: vscode.ExtensionContext,
  baseUrl: string,
): vscode.WebviewPanel {
  const panel = vscode.window.createWebviewPanel(
    "dadrock",
    "Dadrock",
    vscode.ViewColumn.One,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
    },
  );

  panel.webview.html = renderHtml(panel.webview, withVscodeParam(baseUrl));

  panel.webview.onDidReceiveMessage(
    async (msg) => {
      if (msg.command === "openFile") {
        await openFile(msg.path, msg.line);
      }
    },
    undefined,
    context.subscriptions,
  );

  const themeSubscription = vscode.window.onDidChangeActiveColorTheme(() => {
    panel.webview.postMessage({ command: "refreshTheme" });
  });
  panel.onDidDispose(() => themeSubscription.dispose());

  return panel;
}

async function openFile(filePath: string, line?: number): Promise<void> {
  try {
    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(filePath));
    const opts: vscode.TextDocumentShowOptions = {};
    if (line != null && line > 0) {
      const pos = new vscode.Position(line - 1, 0);
      opts.selection = new vscode.Range(pos, pos);
    }
    await vscode.window.showTextDocument(doc, opts);
  } catch (e: unknown) {
    const errMsg = e instanceof Error ? e.message : String(e);
    vscode.window.showErrorMessage(`Could not open file: ${errMsg}`);
  }
}

function withVscodeParam(baseUrl: string): string {
  const url = new URL(baseUrl);
  url.searchParams.set("vscode", "1");
  return url.toString();
}

function renderHtml(webview: vscode.Webview, iframeUrl: string): string {
  const nonce = getNonce();
  const csp =
    `default-src 'none'; ` +
    `style-src ${webview.cspSource} 'unsafe-inline'; ` +
    `script-src 'nonce-${nonce}'; ` +
    `frame-src http://127.0.0.1:*;`;

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="${csp}">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Dadrock</title>
  <style>
    html, body, iframe {
      width: 100%;
      height: 100%;
      margin: 0;
      padding: 0;
      border: 0;
      overflow: hidden;
      background: var(--vscode-editor-background);
      color: var(--vscode-foreground);
    }
  </style>
</head>
<body>
  <iframe id="dadrock-frame" src="${iframeUrl}" sandbox="allow-scripts allow-same-origin allow-forms allow-downloads"></iframe>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();
    const frame = document.getElementById('dadrock-frame');
    const themeVars = [
      '--vscode-editor-background',
      '--vscode-sideBar-background',
      '--vscode-editorWidget-background',
      '--vscode-foreground',
      '--vscode-descriptionForeground',
      '--vscode-panel-border',
      '--vscode-widget-border',
      '--vscode-focusBorder',
      '--vscode-textLink-foreground',
      '--vscode-textLink-activeForeground',
      '--vscode-errorForeground',
      '--vscode-editorWarning-foreground',
      '--vscode-testing-iconPassed',
      '--vscode-charts-green',
      '--vscode-input-background',
      '--vscode-input-foreground',
      '--vscode-input-border',
      '--vscode-button-background',
      '--vscode-button-foreground',
      '--vscode-button-hoverBackground',
      '--vscode-list-hoverBackground',
      '--vscode-list-activeSelectionBackground',
      '--vscode-list-activeSelectionForeground',
      '--vscode-textCodeBlock-background',
      '--vscode-font-family',
      '--vscode-editor-font-family',
      '--vscode-font-size'
    ];

    function currentTheme() {
      const body = document.body;
      if (body.classList.contains('vscode-high-contrast') || body.classList.contains('vscode-high-contrast-light')) return 'hc';
      if (body.classList.contains('vscode-light')) return 'light';
      return 'dark';
    }

    function sendTheme() {
      const styles = getComputedStyle(document.body);
      const vars = {};
      for (const name of themeVars) {
        const value = styles.getPropertyValue(name).trim();
        if (value) vars[name] = value;
      }
      frame.contentWindow?.postMessage({ kind: 'vscode-theme', theme: currentTheme(), vars }, '*');
    }

    frame.addEventListener('load', sendTheme);
    window.addEventListener('message', (event) => {
      const data = event.data || {};
      if (data.kind === 'ready') sendTheme();
      if (data.kind === 'openFile' || data.command === 'openFile') {
        vscode.postMessage({ command: 'openFile', path: data.path, line: data.line });
      }
      if (data.command === 'refreshTheme') sendTheme();
    });
  </script>
</body>
</html>`;
}

function getNonce(): string {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  let nonce = "";
  for (let i = 0; i < 32; i++) {
    nonce += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return nonce;
}
