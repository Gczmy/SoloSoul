import { describe, expect, it } from 'vitest';
import type { MarketPluginInfo } from './plugin';
import { sortPluginsByName } from './pluginOrdering';

function plugin(pluginId: string, name: string, zhName?: string): MarketPluginInfo {
  return {
    pluginId,
    hasUpdate: false,
    isCompatible: true,
    tier: 'p1',
    category: 'productivity',
    registryEntry: {
      id: pluginId,
      name,
      author: 'SoloSoul',
      description: '',
      latestVersion: '1.0.0',
      minCoreVersion: '1.0.0',
      wasmHashSha256: '',
      permissions: [],
      categories: [],
      params: [],
      ...(zhName ? { i18n: { zh: { name: zhName, description: '' } } } : {}),
    },
  };
}

describe('插件名称排序', () => {
  it('忽略后端返回顺序，以显示名称和插件 ID 固定顺序，且不修改源列表', () => {
    const plugins = [
      plugin('z', 'Tool 10'),
      plugin('c', 'beta'),
      plugin('b', 'alpha'),
      plugin('a', 'Alpha'),
      plugin('d', 'Tool 2'),
    ];
    const original = [...plugins];
    const expected = ['a', 'b', 'c', 'd', 'z'];
    for (let start = 0; start < plugins.length; start++) {
      const reordered = [...plugins.slice(start), ...plugins.slice(0, start)].reverse();
      expect(sortPluginsByName(reordered, 'en-US').map((p) => p.pluginId)).toEqual(expected);
    }
    expect(sortPluginsByName(plugins, 'en-US').map((p) => p.pluginId)).toEqual(expected);
    expect(plugins).toEqual(original);
  });

  it('中文按本地化名称的拼音排序，切换英文后按英文名称排序', () => {
    const plugins = [
      plugin('watermark', 'A Watermark', '水印'),
      plugin('guardian', 'B Guardian', '到期提醒'),
      plugin('address', 'C Address', '地址格式化'),
    ];
    expect(sortPluginsByName(plugins, 'zh-CN').map((p) => p.pluginId)).toEqual([
      'guardian',
      'address',
      'watermark',
    ]);
    expect(sortPluginsByName(plugins, 'en-US').map((p) => p.pluginId)).toEqual([
      'watermark',
      'guardian',
      'address',
    ]);
  });

  it('没有翻译时使用默认名称，安装状态变化不改变名称顺序', () => {
    const plugins = [plugin('b', 'Beta'), plugin('a', 'Alpha')];
    expect(sortPluginsByName(plugins, 'zh-CN').map((p) => p.pluginId)).toEqual(['a', 'b']);
    plugins[0].installedVersion = '1.0.0';
    expect(sortPluginsByName(plugins, 'zh-CN').map((p) => p.pluginId)).toEqual(['a', 'b']);
  });
});
