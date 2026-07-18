import init, { Scene } from 'webgal-language-service';

let currentScene: Scene | null = null;
let lastParseTime: number | null = null;

export interface WasmModule {
    // 场景管理
    updateScene: (text: string) => void;
    getSentences: () => unknown[];
    getLastParseTime: () => number | null;

    // 语言服务
    highlight: () => Uint32Array;
    highlightTokenTypes: () => string[];
    complete: (line: number, character: number) => unknown[];
    diagnose: () => unknown[];
    format: () => unknown[];
}

let wasmPromise: Promise<WasmModule> | null = null;

export function loadWasm(): Promise<WasmModule> {
    if (wasmPromise === null) {
        wasmPromise = init().then(() => ({
            updateScene: (text: string) => {
                if (currentScene) {
                    currentScene.free();
                }
                const start = performance.now();
                currentScene = new Scene(text);
                const elapsed = performance.now() - start;
                lastParseTime = elapsed;
            },
            getSentences: () => {
                if (!currentScene) return [];
                return currentScene.sentences();
            },
            getLastParseTime: () => lastParseTime,
            highlight: () => {
                if (!currentScene) return new Uint32Array();
                return currentScene.highlight();
            },
            highlightTokenTypes: () => Scene.highlight_token_types(),
            complete: (line: number, character: number) => {
                if (!currentScene) return [];
                return currentScene.complete(line, character);
            },
            diagnose: () => {
                if (!currentScene) return [];
                return currentScene.diagnose();
            },
            format: () => {
                if (!currentScene) return [];
                return currentScene.format();
            },
        }));
    }
    return wasmPromise!;
}
