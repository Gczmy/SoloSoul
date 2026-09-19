import type { MarketPluginInfo } from './plugin';

const nameCollators = {
  zh: new Intl.Collator('zh-CN-u-co-pinyin', { sensitivity: 'base', numeric: true }),
  en: new Intl.Collator('en', { sensitivity: 'base', numeric: true }),
};

/** 按界面显示名称排序；不依赖后端 HashMap 的遍历顺序，也不修改共享列表。 */
export function sortPluginsByName(
  plugins: readonly MarketPluginInfo[],
  language: string,
): MarketPluginInfo[] {
  const locale = language.startsWith('zh') ? 'zh' : 'en';
  const collator = nameCollators[locale];
  return [...plugins].sort((a, b) => {
    const aName = a.registryEntry.i18n?.[locale]?.name ?? a.registryEntry.name;
    const bName = b.registryEntry.i18n?.[locale]?.name ?? b.registryEntry.name;
    const byName = collator.compare(aName, bName);
    // 重名或仅大小写不同的名称仍须稳定，不能退回到输入顺序。
    return byName || (a.pluginId < b.pluginId ? -1 : a.pluginId > b.pluginId ? 1 : 0);
  });
}
