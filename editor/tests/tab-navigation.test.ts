import { describe, expect, it } from 'vitest';

import { cycleRecentTab, orderTabs, recentTabIds, rememberTabActivation } from '../src/tabs/order';
import { makeConfigTab, makeResourceTab, makeSceneTab, makeWorkbenchTab } from '../src/tabs/model';

describe('tab navigation helpers', () => {
  it('最近使用的标签页会被排到历史末尾，避免重复记录', () => {
    const history = rememberTabActivation(['scene:a', 'scene:b', 'scene:a'], 'scene:b');
    expect(history).toEqual(['scene:a', 'scene:b']);
  });

  it('按最近使用顺序切换, 并在边界循环', () => {
    const tabs = [makeSceneTab('a', 'A'), makeSceneTab('b', 'B'), makeSceneTab('c', 'C'), makeWorkbenchTab()];
    const recent = recentTabIds(tabs, ['scene:b', 'scene:c', 'scene:a']);

    expect(recent).toEqual(['scene:a', 'scene:c', 'scene:b', 'voice-workbench']);
    expect(cycleRecentTab(recent, 'scene:a', 1)).toBe('scene:c');
    expect(cycleRecentTab(recent, 'scene:c', 1)).toBe('scene:b');
    expect(cycleRecentTab(recent, 'scene:a', -1)).toBe('voice-workbench');
    expect(cycleRecentTab(recent, 'scene:a', 1)).toBe('scene:c');
  });

  it('跳过已关闭和重复的历史项, 并把未记录的标签补到末尾', () => {
    const tabs = [makeSceneTab('a', 'A'), makeSceneTab('b', 'B')];
    expect(recentTabIds(tabs, ['scene:a', 'scene:closed', 'scene:a'])).toEqual(['scene:a', 'scene:b']);
  });

  it('没有当前选项卡时从对应方向的边界开始', () => {
    const ids = ['scene:a', 'scene:b'];
    expect(cycleRecentTab(ids, null, 1)).toBe('scene:a');
    expect(cycleRecentTab(ids, null, -1)).toBe('scene:b');
  });

  it('整理时工作台优先、start.txt 置于场景首位、配置先于资源', () => {
    const tabs = [
      makeSceneTab('C:/p/game/scene/other.txt', 'other.txt'),
      makeSceneTab('C:/p/game/scene/start.txt', 'start.txt'),
      makeResourceTab('z-text', 'Text 10', 'text'),
      makeResourceTab('audio', 'Audio', 'audio'),
      makeResourceTab('a-text', 'Text 2', 'text'),
      makeConfigTab('config', 'Config'),
      makeWorkbenchTab(),
    ];

    expect(orderTabs(tabs).map((tab) => tab.id)).toEqual([
      'voice-workbench',
      'scene:C:/p/game/scene/start.txt',
      'scene:C:/p/game/scene/other.txt',
      'config:config',
      'resource:audio',
      'resource:a-text',
      'resource:z-text',
    ]);
  });
});
