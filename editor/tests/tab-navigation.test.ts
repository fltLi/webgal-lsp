import { describe, expect, it } from 'vitest';

import { cycleTabLeft, cycleTabRight, orderTabs, rememberTabActivation } from '../src/tabs/order';
import { makeConfigTab, makeResourceTab, makeSceneTab, makeWorkbenchTab } from '../src/tabs/model';

describe('tab navigation helpers', () => {
  it('最近使用的标签页会被排到历史末尾，避免重复记录', () => {
    const history = rememberTabActivation(['scene:a', 'scene:b', 'scene:a'], 'scene:b');
    expect(history).toEqual(['scene:a', 'scene:b']);
  });

  it('在选项卡列表中向右循环切换时会回到起点', () => {
    const tabs = [
      makeSceneTab('a', 'A'),
      makeSceneTab('b', 'B'),
      makeSceneTab('c', 'C'),
      makeWorkbenchTab(),
    ];

    expect(cycleTabRight(tabs, 'scene:a')).toBe('scene:b');
    expect(cycleTabRight(tabs, 'scene:c')).toBe('voice-workbench');
    expect(cycleTabRight(tabs, 'voice-workbench')).toBe('scene:a');
    expect(cycleTabLeft(tabs, 'scene:a')).toBe('voice-workbench');
    expect(cycleTabLeft(tabs, 'scene:b')).toBe('scene:a');
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
