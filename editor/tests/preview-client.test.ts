import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  addStaticSite: vi.fn(async () => 'site-id'),
  sendPreviewCommand: vi.fn(async (_request: string) => undefined),
  startPreviewServer: vi.fn(async (_host: string, _port: number, _onMessage: (message: string) => void) =>
    'http://127.0.0.1:8899'
  ),
}));

vi.mock('../src/commands/server', () => ({
  addStaticSite: mocks.addStaticSite,
  sendPreviewCommand: mocks.sendPreviewCommand,
  startPreviewServer: mocks.startPreviewServer,
  setEmbeddedPreviewLaunchId: vi.fn(),
  setPreviewMuted: vi.fn(),
}));

import { previewClient } from '../src/preview/client';

describe('previewClient.syncScene', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    previewClient.resetSite();
  });

  it('立即结算当前语句，避免非阻塞演出在实时预览中持续运行', async () => {
    await previewClient.ensureSite('C:\\project');
    await previewClient.syncScene('C:\\project\\game\\scene\\start.txt', 5);

    const request = JSON.parse(mocks.sendPreviewCommand.mock.calls[0]?.[0] ?? '{}') as {
      type?: string;
      payload?: { sceneName?: string; sentenceId?: number; settleMode?: string };
    };
    expect(request).toMatchObject({
      type: 'preview.command.sync-scene',
      payload: { sceneName: 'start.txt', sentenceId: 5, settleMode: 'immediate' },
    });
  });
});