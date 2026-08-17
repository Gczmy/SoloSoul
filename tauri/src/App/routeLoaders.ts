// P015-R5: 页面 chunk loader 单一真相源——供路由懒加载（routes.tsx）、登录后后台预取
// （AppRoutes useRoutePrefetch）与导航悬停/触摸预取（NavButton / MobileBottomNav）共用。
// 本模块只含动态 import()，零静态依赖，避免布局组件为预取而耦合 react-router / stores。

export const loadHomePage = () => import('@/pages/home/HomePage');
export const loadSettingsPage = () => import('@/pages/settings/SettingsPage');
export const loadSecuritySettingsPage = () => import('@/pages/settings/SecuritySettingsPage');
export const loadAccountSettingsPage = () => import('@/pages/settings/AccountSettingsPage');
export const loadDataManagementPage = () => import('@/pages/settings/DataManagementPage');
export const loadTrashPage = () => import('@/pages/settings/TrashPage');
export const loadObjectWorkspacePage = () => import('@/pages/workspace/ObjectWorkspacePage');
export const loadObjectEditorPage = () => import('@/pages/editor/ObjectEditorPage');
export const loadExportImportPage = () => import('@/pages/settings/ExportImportPage');
export const loadSearchPage = () => import('@/pages/search/SearchPage');
export const loadOperationLogPage = () => import('@/pages/settings/OperationLogPage');
export const loadAboutPage = () => import('@/pages/system/AboutPage');
export const loadDebugLogPage = () => import('@/pages/system/DebugLogPage');
export const loadAppearanceSettingsPage = () => import('@/pages/settings/AppearanceSettingsPage');
export const loadBackupConfigPage = () => import('@/pages/settings/BackupConfigPage');
export const loadPluginGatePage = () => import('@/pages/ai/PluginGatePage');
export const loadLlmChatPage = () => import('@/pages/ai/LlmChatPage');
export const loadLlmConfigPage = () => import('@/pages/ai/LlmConfigPage');
export const loadTemplateManagerPage = () => import('@/pages/settings/TemplateManagerPage');
export const loadOcrSettingsPage = () => import('@/pages/settings/OcrSettingsPage');
export const loadGlobalAttachmentManager = () =>
  import('@/pages/settings/GlobalAttachmentManager');
export const loadVaultDirectoryPage = () => import('@/pages/settings/VaultDirectoryPage');
export const loadLlmStatsPage = () => import('@/pages/ai/LlmStatsPage');
export const loadHelpPage = () => import('@/pages/help/HelpPage');
export const loadScanLocalPage = () => import('@/pages/scan/ScanLocalPage');
export const loadOcrPage = () => import('@/pages/scan/OcrPage');
export const loadHistoryPage = () => import('@/pages/editor/HistoryPage');
export const loadSyncPage = () => import('@/pages/sync/SyncPage');

/** 受保护路由页面加载器集合：登录后在后台分批预取，消除首次进入未访问页面的整窗空白。 */
export const routeLoaders: Array<() => Promise<unknown>> = [
  loadHomePage,
  loadSettingsPage,
  loadSecuritySettingsPage,
  loadAccountSettingsPage,
  loadDataManagementPage,
  loadTrashPage,
  loadObjectWorkspacePage,
  loadObjectEditorPage,
  loadExportImportPage,
  loadSearchPage,
  loadOperationLogPage,
  loadAboutPage,
  loadDebugLogPage,
  loadAppearanceSettingsPage,
  loadBackupConfigPage,
  loadPluginGatePage,
  loadLlmChatPage,
  loadLlmConfigPage,
  loadTemplateManagerPage,
  loadOcrSettingsPage,
  loadGlobalAttachmentManager,
  loadVaultDirectoryPage,
  loadLlmStatsPage,
  loadHelpPage,
  loadScanLocalPage,
  loadOcrPage,
  loadHistoryPage,
  loadSyncPage,
];

/** pathname → loader 映射（悬停/触摸预取入口）。覆盖全部受保护路由；query/hash 剥离与
 *  动态段回退在 resolveRouteLoader 中处理。 */
const routePathToLoader: Record<string, () => Promise<unknown>> = {
  '/': loadHomePage,
  '/search': loadSearchPage,
  '/settings': loadSettingsPage,
  '/settings/appearance': loadAppearanceSettingsPage,
  '/settings/security': loadSecuritySettingsPage,
  '/settings/account': loadAccountSettingsPage,
  '/settings/export-import': loadExportImportPage,
  '/settings/data': loadDataManagementPage,
  '/settings/trash': loadTrashPage,
  '/settings/operation-log': loadOperationLogPage,
  '/settings/backup': loadBackupConfigPage,
  '/about': loadAboutPage,
  '/debug-log': loadDebugLogPage,
  '/plugins': loadPluginGatePage,
  '/settings/templates': loadTemplateManagerPage,
  '/settings/attachments': loadGlobalAttachmentManager,
  '/settings/vault-directory': loadVaultDirectoryPage,
  '/settings/ocr': loadOcrSettingsPage,
  '/settings/llm': loadLlmConfigPage,
  '/settings/llm/stats': loadLlmStatsPage,
  '/llm-chat': loadLlmChatPage,
  '/local-import': loadScanLocalPage,
  '/history': loadHistoryPage,
  '/ocr': loadOcrPage,
  '/sync': loadSyncPage,
  '/help': loadHelpPage,
  '/workspace': loadObjectWorkspacePage,
  '/editor': loadObjectEditorPage,
};

/** 将任意导航路径解析为对应页面 chunk loader；query/hash 剥离，动态段回退到页面 loader。 */
export function resolveRouteLoader(path: string): (() => Promise<unknown>) | undefined {
  const pathname = path.split(/[?#]/)[0];
  if (routePathToLoader[pathname]) return routePathToLoader[pathname];
  if (pathname.startsWith('/workspace/custom/')) return loadObjectWorkspacePage;
  if (pathname.startsWith('/editor/')) return loadObjectEditorPage;
  return undefined;
}

/**
 * 悬停/触摸预取入口（P015-R5）：指针移到/按到导航项时预热目标页面 chunk，
 * 点击导航时 chunk 已命中缓存，消除 React.lazy + Suspense 造成的切页骨架屏。
 * 静默失败——预取失败不影响目标页面的按需加载。
 */
export function prefetchRoute(path: string): void {
  resolveRouteLoader(path)?.().catch(() => {});
}
