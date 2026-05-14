import * as vscode from "vscode";
import { ChildProcess, spawn } from "child_process";
import * as net from "net";

export class DadrockProcess {
  private child: ChildProcess | null = null;
  private _baseUrl: string | null = null;
  private _onExit = new vscode.EventEmitter<number | null>();
  readonly onExit = this._onExit.event;

  get baseUrl(): string | null {
    return this._baseUrl;
  }

  get running(): boolean {
    return this.child !== null && this.child.exitCode === null;
  }

  async start(binaryPath: string, projectRoot: string): Promise<string> {
    if (this.running) {
      return this._baseUrl!;
    }

    const port = await findFreePort();
    const args = ["preview", "--host", "127.0.0.1", "--port", String(port), projectRoot];

    this.child = spawn(binaryPath, args, {
      stdio: ["ignore", "pipe", "pipe"],
    });

    let url: string;
    try {
      url = await this.waitForReady();
      this._baseUrl = url;
    } catch (err) {
      this.stop();
      throw err;
    }

    this.child.on("exit", (code) => {
      this.child = null;
      this._baseUrl = null;
      this._onExit.fire(code);
    });

    return url;
  }

  stop(): void {
    if (this.child) {
      this.child.kill("SIGTERM");
      this.child = null;
      this._baseUrl = null;
    }
  }

  dispose(): void {
    this.stop();
    this._onExit.dispose();
  }

  private waitForReady(): Promise<string> {
    return new Promise((resolve, reject) => {
      const child = this.child;
      const timeout = setTimeout(() => {
        cleanup();
        reject(new Error("dadrock preview did not start within 10 seconds"));
      }, 10_000);

      const stderr = child?.stderr;
      if (!stderr) {
        clearTimeout(timeout);
        reject(new Error("no stderr stream"));
        return;
      }

      let buf = "";
      const onData = (chunk: Buffer) => {
        buf += chunk.toString();
        const match = buf.match(/dadrock-preview: listening on (http:\/\/[^\s]+)/);
        if (match) {
          cleanup();
          resolve(match[1]);
        }
      };

      const onExit = (code: number | null) => {
        cleanup();
        const tail = buf.trim().split("\n").slice(-5).join("\n");
        const detail = tail ? `: ${tail}` : "";
        reject(new Error(`dadrock exited with code ${code} before ready${detail}`));
      };

      const cleanup = () => {
        clearTimeout(timeout);
        stderr.removeListener("data", onData);
        child?.removeListener("exit", onExit);
      };

      stderr.on("data", onData);

      child?.on("exit", onExit);
    });
  }
}

function findFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.listen(0, "127.0.0.1", () => {
      const addr = srv.address() as net.AddressInfo;
      srv.close(() => resolve(addr.port));
    });
    srv.on("error", reject);
  });
}
