import { Navigate } from 'react-router-dom';

// P015: 路由级懒加载——27 个页面全部改为 React.lazy 动态导入，vite 自动按页面分包，
// 移动端首屏只加载登录/首页相关 chunk，进入其他页面时才拉取对应包。
// P015-R: 每个页面拆出独立 loadXxx() 加载器（见 ./routeLoaders），与下方 lazyPage() 及
// routeLoaders（登录后后台预取）共用同一 import 工厂，保证懒加载与预取单真相源。
// P015-R4: lazy(() => loadXxx().then((m) => ({ default: m.Xxx }))) 样板收敛为 lazyPage()，
// 命名导出映射由 keyof 编译期校验，重命名即报错。
import { lazyPage } from './lazyPage';
import {
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
} from './routeLoaders';
import { useAuthStore } from '@/stores/authStore';
import type { ReactNode } from 'react';

const HomePage = lazyPage(loadHomePage, 'HomePage');
const SettingsPage = lazyPage(loadSettingsPage, 'SettingsPage');
const SecuritySettingsPage = lazyPage(loadSecuritySettingsPage, 'SecuritySettingsPage');
const AccountSettingsPage = lazyPage(loadAccountSettingsPage, 'AccountSettingsPage');
const DataManagementPage = lazyPage(loadDataManagementPage, 'DataManagementPage');
const TrashPage = lazyPage(loadTrashPage, 'TrashPage');
const ObjectWorkspacePage = lazyPage(loadObjectWorkspacePage, 'ObjectWorkspacePage');
const ObjectEditorPage = lazyPage(loadObjectEditorPage, 'ObjectEditorPage');
const ExportImportPage = lazyPage(loadExportImportPage, 'ExportImportPage');
const SearchPage = lazyPage(loadSearchPage, 'SearchPage');
const OperationLogPage = lazyPage(loadOperationLogPage, 'OperationLogPage');
const AboutPage = lazyPage(loadAboutPage, 'AboutPage');
const DebugLogPage = lazyPage(loadDebugLogPage, 'DebugLogPage');
const AppearanceSettingsPage = lazyPage(loadAppearanceSettingsPage, 'AppearanceSettingsPage');
const BackupConfigPage = lazyPage(loadBackupConfigPage, 'BackupConfigPage');
const PluginGatePage = lazyPage(loadPluginGatePage, 'PluginGatePage');
const LlmChatPage = lazyPage(loadLlmChatPage, 'LlmChatPage');
const LlmConfigPage = lazyPage(loadLlmConfigPage, 'LlmConfigPage');
const TemplateManagerPage = lazyPage(loadTemplateManagerPage, 'TemplateManagerPage');
const OcrSettingsPage = lazyPage(loadOcrSettingsPage, 'OcrSettingsPage');
const GlobalAttachmentManager = lazyPage(loadGlobalAttachmentManager, 'GlobalAttachmentManager');
const VaultDirectoryPage = lazyPage(loadVaultDirectoryPage, 'VaultDirectoryPage');
const LlmStatsPage = lazyPage(loadLlmStatsPage, 'LlmStatsPage');
const HelpPage = lazyPage(loadHelpPage, 'HelpPage');
const ScanLocalPage = lazyPage(loadScanLocalPage, 'ScanLocalPage');
const OcrPage = lazyPage(loadOcrPage, 'OcrPage');
const HistoryPage = lazyPage(loadHistoryPage, 'HistoryPage');
const SyncPage = lazyPage(loadSyncPage, 'SyncPage');

// 受保护路由页面加载器集合：由 routeLoaders.ts 统一导出，供 AppRoutes 登录后后台预取
// 与导航悬停/触摸预取（prefetchRoute）共用同一 loader。
export { routeLoaders } from './routeLoaders';

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
