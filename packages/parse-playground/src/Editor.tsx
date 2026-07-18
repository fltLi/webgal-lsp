import { useEffect, useRef, useState } from 'react';
import Editor, { type Monaco } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import { type WasmModule } from './wasm';

interface EditorProps {
    value: string;
    onChange: (value: string) => void;
    onCursorChange?: (line: number) => void;
    wasm?: WasmModule | null;
}

// 转换 LSP Diagnostic 为 Monaco IMarkerData
function toMarkerData(diagnostic: any): monaco.editor.IMarkerData {
    const severityMap: Record<number, monaco.MarkerSeverity> = {
        1: monaco.MarkerSeverity.Error,
        2: monaco.MarkerSeverity.Warning,
        3: monaco.MarkerSeverity.Info,
        4: monaco.MarkerSeverity.Hint,
    };
    const severity = severityMap[diagnostic.severity] ?? monaco.MarkerSeverity.Error;

    return {
        severity,
        message: diagnostic.message,
        startLineNumber: diagnostic.range.start.line + 1,
        startColumn: diagnostic.range.start.character + 1,
        endLineNumber: diagnostic.range.end.line + 1,
        endColumn: diagnostic.range.end.character + 1,
        code: typeof diagnostic.code === 'string' ? diagnostic.code : String(diagnostic.code ?? ''),
        source: 'webgal',
    };
}

export function SceneEditor({ value, onChange, onCursorChange, wasm }: EditorProps) {
    const editorRef = useRef<Parameters<NonNullable<Parameters<typeof Editor>[0]['onMount']>>[0] | null>(null);
    const monacoRef = useRef<Monaco | null>(null);
    const [isReady, setIsReady] = useState(false);
    const diagnoseTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const registerProviders = (monaco: Monaco) => {
        const languageId = 'webgal';

        if (!monaco.languages.getLanguages().some((lang) => lang.id === languageId)) {
            monaco.languages.register({
                id: languageId,
                extensions: ['.txt'],
                aliases: ['WebGAL', 'WebGAL Script'],
                mimetypes: ['application/webgalscript'],
            });
            monaco.languages.setLanguageConfiguration(languageId, {
                comments: { lineComment: ';' },
                brackets: [
                    ['{', '}'],
                    ['[', ']'],
                    ['(', ')'],
                ],
                autoClosingPairs: [
                    { open: '{', close: '}' },
                    { open: '[', close: ']' },
                    { open: '(', close: ')' },
                ],
            });
        }

        // 语义高亮
        const tokenTypes = wasm?.highlightTokenTypes?.() ?? [];
        const legend = { tokenTypes, tokenModifiers: [] };
        monaco.languages.registerDocumentSemanticTokensProvider(languageId, {
            getLegend: () => legend,
            provideDocumentSemanticTokens: () => {
                try {
                    const result = wasm?.highlight?.();
                    if (!result || result.length === 0) {
                        return { data: new Uint32Array() };
                    }
                    return { data: result };
                } catch (e) {
                    console.error('Highlight error:', e);
                    return { data: new Uint32Array() };
                }
            },
            releaseDocumentSemanticTokens: () => { /* no-op */ },
        });
    };

    const handleMount = (editor: Parameters<NonNullable<Parameters<typeof Editor>[0]['onMount']>>[0], monaco: Monaco) => {
        editorRef.current = editor;
        monacoRef.current = monaco;
        monaco.editor.setTheme('vs-dark');
        setIsReady(true);

        if (wasm) {
            registerProviders(monaco);
        }

        const model = editor.getModel();
        if (model) {
            monaco.editor.setModelLanguage(model, 'webgal');
        }
    };

    // 光标变化
    useEffect(() => {
        if (!editorRef.current) return;
        const disposable = editorRef.current.onDidChangeCursorPosition((event) => {
            onCursorChange?.(event.position.lineNumber - 1);
        });
        return () => disposable.dispose();
    }, [onCursorChange]);

    // 重新注册 Providers（当 wasm 变化时）
    useEffect(() => {
        if (!isReady || !monacoRef.current || !wasm) return;
        const monaco = monacoRef.current;
        registerProviders(monaco);

        const editor = editorRef.current;
        if (editor) {
            const model = editor.getModel();
            if (model) {
                monaco.editor.setModelLanguage(model, 'plaintext');
                monaco.editor.setModelLanguage(model, 'webgal');
                (editor as any).updateOptions({ 'semanticHighlighting.enabled': true });
            }
        }
    }, [wasm, isReady]);

    // 诊断更新
    useEffect(() => {
        if (!wasm || !monacoRef.current || !editorRef.current) return;
        const model = editorRef.current.getModel();
        if (!model) return;

        // 清除之前的定时器
        if (diagnoseTimerRef.current) {
            clearTimeout(diagnoseTimerRef.current);
            diagnoseTimerRef.current = null;
        }

        // 延迟执行诊断，避免频繁更新
        diagnoseTimerRef.current = setTimeout(() => {
            try {
                const diagnostics = wasm.diagnose() as any[];
                const markers = diagnostics.map(toMarkerData);
                monacoRef.current?.editor.setModelMarkers(model, 'webgal', markers);
            } catch (e) {
                console.error('Diagnose error:', e);
            }
            diagnoseTimerRef.current = null;
        }, 300);
    }, [wasm, value]);

    return (
        <Editor
            height="100%"
            language="webgal"
            theme="vs-dark"
            value={value}
            onChange={(nextValue) => onChange(nextValue ?? '')}
            onMount={handleMount}
            options={{
                minimap: { enabled: false },
                automaticLayout: true,
                fontSize: 14,
                wordWrap: 'on',
                'semanticHighlighting.enabled': true,
            }}
        />
    );
}
