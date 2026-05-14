import * as vscode from "vscode";
import { execFile, execSync } from "child_process";
import { DadrockProcess } from "./dadrock-process";
import { createPanel } from "./webview-panel";

let dadrockProcess: DadrockProcess | null = null;
let panel: vscode.WebviewPanel | null = null;
let statusBar: vscode.StatusBarItem | null = null;

export function activate(context: vscode.ExtensionContext): void {
  statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
  statusBar.command = "dadrock.showPanel";
  statusBar.text = "$(notebook) dadrock";
  context.subscriptions.push(statusBar);

  const register = (cmd: string, fn: () => Promise<void> | void) => {
    context.subscriptions.push(vscode.commands.registerCommand(cmd, fn));
  };

  register("dadrock.start", () => startDadrock(context));
  register("dadrock.stop", stopDadrock);
  register("dadrock.showPanel", () => showPanel(context));
  register("dadrock.restart", () => restartDadrock(context));
  register("dadrock.init", () => initDadrock(context));

  const cfg = vscode.workspace.getConfiguration("dadrock");
  if (cfg.get<boolean>("autoStart")) {
    startDadrock(context);
  }
}

export function deactivate(): void {
  dadrockProcess?.dispose();
  dadrockProcess = null;
}

async function startDadrock(context: vscode.ExtensionContext): Promise<void> {
  if (dadrockProcess?.running) {
    vscode.window.showInformationMessage("Dadrock is already running.");
    showPanel(context);
    return;
  }

  const binaryPath = await findBinary(context);
  if (!binaryPath) return;

  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    vscode.window.showErrorMessage("No workspace folder open.");
    return;
  }

  const proc = new DadrockProcess();
  dadrockProcess = proc;

  proc.onExit((code) => {
    if (dadrockProcess !== proc) return;
    if (statusBar) statusBar.text = "$(notebook) dadrock (stopped)";
    if (code !== null && code !== 0) {
      vscode.window
        .showErrorMessage(`Dadrock exited with code ${code}.`, "Restart")
        .then((choice) => {
          if (choice === "Restart") startDadrock(context);
        });
    }
  });

  try {
    await proc.start(binaryPath, folder.uri.fsPath);
    if (statusBar) {
      statusBar.text = "$(notebook) dadrock";
      statusBar.show();
    }
    showPanel(context);
  } catch (e: unknown) {
    const errMsg = e instanceof Error ? e.message : String(e);
    vscode.window.showErrorMessage(`Failed to start Dadrock: ${errMsg}`);
  }
}

async function restartDadrock(context: vscode.ExtensionContext): Promise<void> {
  stopDadrock();
  await new Promise((resolve) => setTimeout(resolve, 250));
  await startDadrock(context);
}

async function initDadrock(context: vscode.ExtensionContext): Promise<void> {
  const binaryPath = await findBinary(context);
  if (!binaryPath) return;

  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    vscode.window.showErrorMessage("No workspace folder open.");
    return;
  }

  const output = vscode.window.createOutputChannel("Dadrock Init");
  output.show(true);
  output.appendLine(`$ dadrock init ${folder.uri.fsPath}`);

  await new Promise<void>((resolve) => {
    execFile(
      binaryPath,
      ["init", folder.uri.fsPath],
      { encoding: "utf-8" },
      (err: Error | null, stdout: string, stderr: string) => {
        if (stdout) output.append(stdout);
        if (stderr) output.append(stderr);
        if (err) {
          output.appendLine(`\ndadrock init failed: ${err.message}`);
          vscode.window.showErrorMessage("dadrock init failed. See 'Dadrock Init' output for details.");
        } else {
          output.appendLine("\nDone.");
          vscode.window.showInformationMessage("Dadrock initialized.", "Start").then((choice) => {
            if (choice === "Start") startDadrock(context);
          });
        }
        resolve();
      },
    );
  });
}

function stopDadrock(): void {
  dadrockProcess?.dispose();
  dadrockProcess = null;
  panel?.dispose();
  panel = null;
  if (statusBar) statusBar.text = "$(notebook) dadrock (stopped)";
  vscode.window.showInformationMessage("Dadrock stopped.");
}

function showPanel(context: vscode.ExtensionContext): void {
  if (panel) {
    panel.reveal();
    return;
  }
  if (!dadrockProcess?.baseUrl) {
    vscode.window.showInformationMessage("Dadrock is not running. Use 'Dadrock: Start' first.");
    return;
  }
  panel = createPanel(context, dadrockProcess.baseUrl);
  panel.onDidDispose(() => {
    panel = null;
  });
}

async function findBinary(context: vscode.ExtensionContext): Promise<string | null> {
  const cfg = vscode.workspace.getConfiguration("dadrock");
  const explicit = cfg.get<string>("binaryPath", "").trim().replace(/^["']+|["']+$/g, "").trim();
  if (explicit) return explicit;

  const exe = process.platform === "win32" ? "dadrock.exe" : "dadrock";
  const bundled = vscode.Uri.joinPath(context.extensionUri, "bin", exe).fsPath;
  try {
    await vscode.workspace.fs.stat(vscode.Uri.file(bundled));
    if (process.platform !== "win32") {
      const fs = require("fs");
      fs.chmodSync(bundled, 0o755);
    }
    return bundled;
  } catch {
    // Not bundled.
  }

  try {
    const cmd = process.platform === "win32" ? "where dadrock" : "which dadrock";
    const out = execSync(cmd, { encoding: "utf-8" }).split(/\r?\n/)[0].trim();
    if (out) return out;
  } catch {
    // Not on PATH.
  }

  vscode.window.showErrorMessage(
    "Could not find the `dadrock` binary. Install it or set `dadrock.binaryPath` in settings.",
  );
  return null;
}
