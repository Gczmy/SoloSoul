import { Navigate } from 'react-router-dom';
import { lazy } from 'react';

// P015: 路由级懒加载——27 个页面全部改为 React.lazy 动态导入，vite 自动按页面分包，
// 移动端首屏只加载登录/首页相关 chunk，进入其他页面时才拉取对应包。
// P015-R: 每个页面拆出独立 loadXxx() 加载器，与下方 lazy() 及 routeLoaders（登录后
// 后台预取）共用同一 import 工厂，保证懒加载与预取单真相源、chunk 划分不变。
const loadHomePage = () => import('@/pages/home/HomePage');
const HomePage = lazy(() => loadHomePage().then((m) => ({ default: m.HomePage })));
const loadSettingsPage = () => import('@/pages/settings/SettingsPage');
const SettingsPage = lazy(() => loadSettingsPage().then((m) => ({ default: m.SettingsPage })));
const loadSecuritySettingsPage = () => import('@/pages/settings/SecuritySettingsPage');
const SecuritySettingsPage = lazy(() =>
  loadSecuritySettingsPage().then((m) => ({
    default: m.SecuritySettingsPage,
  })),
);
const loadAccountSettingsPage = () => import('@/pages/settings/AccountSettingsPage');
const AccountSettingsPage = lazy(() =>
  loadAccountSettingsPage().then((m) => ({ default: m.AccountSettingsPage })),
);
const loadDataManagementPage = () => import('@/pages/settings/DataManagementPage');
const DataManagementPage = lazy(() =>
  loadDataManagementPage().then((m) => ({ default: m.DataManagementPage })),
);
const loadTrashPage = () => import('@/pages/settings/TrashPage');
const TrashPage = lazy(() => loadTrashPage().then((m) => ({ default: m.TrashPage })));
const loadObjectWorkspacePage = () => import('@/pages/workspace/ObjectWorkspacePage');
const ObjectWorkspacePage = lazy(() =>
  loadObjectWorkspacePage().then((m) => ({ default: m.ObjectWorkspacePage })),
);
const loadObjectEditorPage = () => import('@/pages/editor/ObjectEditorPage');
const ObjectEditorPage = lazy(() =>
  loadObjectEditorPage().then((m) => ({ default: m.ObjectEditorPage })),
);
const loadExportImportPage = () => import('@/pages/settings/ExportImportPage');
const ExportImportPage = lazy(() =>
  loadExportImportPage().then((m) => ({ default: m.ExportImportPage })),
);
const loadSearchPage = () => import('@/pages/search/SearchPage');
const SearchPage = lazy(() => loadSearchPage().then((m) => ({ default: m.SearchPage })));
const loadOperationLogPage = () => import('@/pages/settings/OperationLogPage');
const OperationLogPage = lazy(() =>
  loadOperationLogPage().then((m) => ({ default: m.OperationLogPage })),
);
const loadAboutPage = () => import('@/pages/system/AboutPage');
const AboutPage = lazy(() => loadAboutPage().then((m) => ({ default: m.AboutPage })));
const loadDebugLogPage = () => import('@/pages/system/DebugLogPage');
const DebugLogPage = lazy(() => loadDebugLogPage().then((m) => ({ default: m.DebugLogPage })));
const loadAppearanceSettingsPage = () => import('@/pages/settings/AppearanceSettingsPage');
const AppearanceSettingsPage = lazy(() =>
  loadAppearanceSettingsPage().then((m) => ({
    default: m.AppearanceSettingsPage,
  })),
);
const loadBackupConfigPage = () => import('@/pages/settings/BackupConfigPage');
const BackupConfigPage = lazy(() =>
  loadBackupConfigPage().then((m) => ({ default: m.BackupConfigPage })),
);
const loadPluginGatePage = () => import('@/pages/ai/PluginGatePage');
const PluginGatePage = lazy(() => loadPluginGatePage().then((m) => ({ default: m.PluginGatePage })));
const loadLlmChatPage = () => import('@/pages/ai/LlmChatPage');
const LlmChatPage = lazy(() => loadLlmChatPage().then((m) => ({ default: m.LlmChatPage })));
const loadLlmConfigPage = () => import('@/pages/ai/LlmConfigPage');
const LlmConfigPage = lazy(() => loadLlmConfigPage().then((m) => ({ default: m.LlmConfigPage })));
const loadTemplateManagerPage = () => import('@/pages/settings/TemplateManagerPage');
const TemplateManagerPage = lazy(() =>
  loadTemplateManagerPage().then((m) => ({ default: m.TemplateManagerPage })),
);
const loadOcrSettingsPage = () => import('@/pages/settings/OcrSettingsPage');
const OcrSettingsPage = lazy(() =>
  loadOcrSettingsPage().then((m) => ({ default: m.OcrSettingsPage })),
);
const loadGlobalAttachmentManager = () => import('@/pages/settings/GlobalAttachmentManager');
const GlobalAttachmentManager = lazy(() =>
  loadGlobalAttachmentManager().then((m) => ({
    default: m.GlobalAttachmentManager,
  })),
);
const loadVaultDirectoryPage = () => import('@/pages/settings/VaultDirectoryPage');
const VaultDirectoryPage = lazy(() =>
  loadVaultDirectoryPage().then((m) => ({ default: m.VaultDirectoryPage })),
);
const loadLlmStatsPage = () => import('@/pages/ai/LlmStatsPage');
const LlmStatsPage = lazy(() => loadLlmStatsPage().then((m) => ({ default: m.LlmStatsPage })));
const loadHelpPage = () => import('@/pages/help/HelpPage');
const HelpPage = lazy(() => loadHelpPage().then((m) => ({ default: m.HelpPage })));
const loadScanLocalPage = () => import('@/pages/scan/ScanLocalPage');
const ScanLocalPage = lazy(() => loadScanLocalPage().then((m) => ({ default: m.ScanLocalPage })));
const loadOcrPage = () => import('@/pages/scan/OcrPage');
const OcrPage = lazy(() => loadOcrPage().then((m) => ({ default: m.OcrPage })));
const loadHistoryPage = () => import('@/pages/editor/HistoryPage');
const HistoryPage = lazy(() => loadHistoryPage().then((m) => ({ default: m.HistoryPage })));
const loadSyncPage = () => import('@/pages/sync/SyncPage');
const SyncPage = lazy(() => loadSyncPage().then((m) => ({ default: m.SyncPage })));
import { useAuthStore } from '@/stores/authStore';
import type { ReactNode } from 'react';

/** 受保护路由页面加载器集合：登录后在后台分批预取（见 AppRoutes useRoutePrefetch），
 *  消除桌面端首次进入未访问页面时拉取 chunk（含共享依赖 chunk）造成的整窗空白。 */
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

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  if (!isAuthenticated) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

export interface RouteConfig {
  path: string;
  element: ReactNode;
}

export const protectedRoutes: RouteConfig[] = [
  { path: '/', element: <HomePage /> },
  { path: '/search', element: <SearchPage /> },
  { path: '/settings', element: <SettingsPage /> },
  { path: '/settings/appearance', element: <AppearanceSettingsPage /> },
  { path: '/settings/security', element: <SecuritySettingsPage /> },
  { path: '/settings/account', element: <AccountSettingsPage /> },
  { path: '/settings/export-import', element: <ExportImportPage /> },
  { path: '/settings/data', element: <DataManagementPage /> },
  { path: '/settings/trash', element: <TrashPage /> },
  { path: '/settings/operation-log', element: <OperationLogPage /> },
  { path: '/settings/backup', element: <BackupConfigPage /> },
  { path: '/about', element: <AboutPage /> },
  { path: '/debug-log', element: <DebugLogPage /> },
  { path: '/plugins', element: <PluginGatePage /> },
  { path: '/settings/templates', element: <TemplateManagerPage /> },
  { path: '/settings/attachments', element: <GlobalAttachmentManager /> },
  { path: '/settings/vault-directory', element: <VaultDirectoryPage /> },
  { path: '/settings/ocr', element: <OcrSettingsPage /> },
  { path: '/settings/llm', element: <LlmConfigPage /> },
  { path: '/settings/llm/stats', element: <LlmStatsPage /> },
  { path: '/llm-chat', element: <LlmChatPage /> },
  { path: '/local-import', element: <ScanLocalPage /> },
  { path: '/history', element: <HistoryPage /> },
  { path: '/ocr', element: <OcrPage /> },
  { path: '/sync', element: <SyncPage /> },
  { path: '/help', element: <HelpPage /> },
  { path: '/workspace', element: <ObjectWorkspacePage /> },
  { path: '/editor', element: <ObjectEditorPage /> },
  { path: '/editor/:objectId', element: <ObjectEditorPage /> },
  { path: '/workspace/custom/:pageId', element: <ObjectWorkspacePage /> },
];
