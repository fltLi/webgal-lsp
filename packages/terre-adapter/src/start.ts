import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import { StreamMessageReader, StreamMessageWriter } from 'vscode-jsonrpc/node';

const PLATFORM_BIN_MAP: Record<string, string> = {
  linux: 'webgal-language-server-linux-x86_64',
  darwin: 'webgal-language-server-macos-x86_64',
  win32: 'webgal-language-server-windows-x86_64.exe',
};
if (process.platform === 'darwin' && process.arch === 'arm64') {
  PLATFORM_BIN_MAP.darwin = 'webgal-language-server-macos-aarch64';
}

let proc: ChildProcess | null = null;

export default {
  name: 'WebGAL Language Server',

  async start() {
    try {
      const binName = PLATFORM_BIN_MAP[process.platform];
      if (!binName) {
        throw new Error(`Unsupported platform: ${process.platform}`);
      }
      const binPath = path.join(__dirname, 'bin', binName);
      console.log(`[Adapter] Starting: ${binPath}`);

      proc = spawn(binPath, [], {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: { ...process.env },
      });

      proc.stderr!.pipe(process.stderr);
      proc.on('exit', code => console.log(`[Adapter] Exited with ${code}`));
      proc.on('error', err => console.error(`[Adapter] Error: ${err}`));

      return {
        reader: new StreamMessageReader(proc.stdout!),
        writer: new StreamMessageWriter(proc.stdin!),
      };
    } catch (err) {
      console.error('[Adapter] Failed to start:', err);
      throw err;
    }
  },

  async stop() {
    if (proc) {
      proc.kill();
      proc = null;
      console.log('[Adapter] Stopped');
    }
  },
};
