// 资源工作区: 文本资源使用 Monaco, 媒体资源使用原生展示控件。

import { useEffect, useState } from 'react';

import { projectRelativePath } from '../fileops';
import { previewClient } from '../preview/client';
import { useAppStore, type OpenDocument } from '../state/store';
import type { ResourceTab as ResourceTabModel } from '../tabs/model';
import { CodeEditor } from './CodeEditor';

export function ResourceTab({ tab, document }: { tab: ResourceTabModel; document: OpenDocument | null }) {
  const projectPath = useAppStore((state) => state.projectPath);
  const settings = useAppStore((state) => state.settings);
  const [assetBase, setAssetBase] = useState<string | null>(null);

  useEffect(() => {
    if (tab.resourceKind === 'text' || !projectPath) return;
    let cancelled = false;
    void previewClient.ensureSite(projectPath, settings.enginePath ?? undefined).then((url) => {
      if (!cancelled) setAssetBase(url);
    });
    return () => {
      cancelled = true;
    };
  }, [projectPath, settings.enginePath, tab.resourceKind]);

  if (tab.resourceKind === 'text') {
    return document ? <CodeEditor doc={document} /> : <div className="editor-empty">正在读取资源…</div>;
  }

  if (!projectPath || !assetBase) return <div className="editor-empty">正在加载资源…</div>;
  const relative = projectRelativePath(projectPath, tab.path).replace(/\\/g, '/');
  const resourcePath = relative.toLowerCase().startsWith('game/') ? relative.slice('game/'.length) : relative;
  const url = `${assetBase}game/${resourcePath.split('/').map(encodeURIComponent).join('/')}`;

  return (
    <div className="resource-workspace">
      {tab.resourceKind === 'image' ? <img src={url} alt={tab.title} /> : null}
      {tab.resourceKind === 'audio' ? <audio src={url} controls autoPlay={false} /> : null}
      {tab.resourceKind === 'video' ? <video src={url} controls autoPlay={false} /> : null}
    </div>
  );
}
