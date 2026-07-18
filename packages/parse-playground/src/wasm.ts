import init, { Scene } from 'webgal-language-service';

let currentScene: Scene | null = null;
let lastParseTime: number | null = null;

export interface WasmModule {
    highlight_token_types: () => string[];
    updateScene: (text: string) => void;
    highlightScene: () => Uint32Array;
    getSentences: () => unknown[];
    getLastParseTime: () => number | null;
}

let wasmPromise: Promise<WasmModule> | null = null;

export function loadWasm(): Promise<WasmModule> {
    if (wasmPromise === null) {
        wasmPromise = init().then(() => ({
            highlight_token_types: () => Scene.highlight_token_types(),
            updateScene: (text: string) => {
                if (currentScene) {
                    currentScene.free();
                }
                const start = performance.now();
                currentScene = new Scene(text);
                const elapsed = performance.now() - start;
                lastParseTime = elapsed;
            },
            highlightScene: () => {
                if (!currentScene) {
                    return new Uint32Array();
                }
                return currentScene.highlight();
            },
            getSentences: () => {
                if (!currentScene) {
                    return [];
                }
                return currentScene.sentences();
            },
            getLastParseTime: () => lastParseTime,
        }));
    }
    return wasmPromise!;
}
