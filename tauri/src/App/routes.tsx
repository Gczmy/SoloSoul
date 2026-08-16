import { Navigate } from 'react-router-dom';
import { lazy } from 'react';

// P015: 路由级懒加载——27 个页面全部改为 React.lazy 动态导入，vite 自动按页面分包，
// 移动端首屏只加载登录/首页相关 chunk，进入其他页面时才拉取对应包。
const HomePage = lazy(() => import('@/pages/home/HomePage').then((m) => ({ default: m.HomePage })));
const SettingsPage = lazy(() =>
  import('@/pages/settings/SettingsPage').then((m) => ({ default: m.SettingsPage })),
);
const SecuritySettingsPage = lazy(() =>
  import('@/pages/settings/SecuritySettingsPage').then((m) => ({
    default: m.SecuritySettingsPage,
  })),
);
const AccountSettingsPage = lazy(() =>
  import('@/pages/settings/AccountSettingsPage').then((m) => ({ default: m.AccountSettingsPage })),
);
const DataManagementPage = lazy(() =>
  import('@/pages/settings/DataManagementPage').then((m) => ({ default: m.DataManagementPage })),
);
const TrashPage = lazy(() =>
  import('@/pages/settings/TrashPage').then((m) => ({ default: m.TrashPage })),
);
const ObjectWorkspacePage = lazy(() =>
  import('@/pages/workspace/ObjectWorkspacePage').then((m) => ({ default: m.ObjectWorkspacePage })),
);
const ObjectEditorPage = lazy(() =>
  import('@/pages/editor/ObjectEditorPage').then((m) => ({ default: m.ObjectEditorPage })),
);
const ExportImportPage = lazy(() =>
  import('@/pages/settings/ExportImportPage').then((m) => ({ default: m.ExportImportPage })),
);
const SearchPage = lazy(() =>
  import('@/pages/search/SearchPage').then((m) => ({ default: m.SearchPage })),
);
const OperationLogPage = lazy(() =>
  import('@/pages/settings/OperationLogPage').then((m) => ({ default: m.OperationLogPage })),
);
const AboutPage = lazy(() =>
  import('@/pages/system/AboutPage').then((m) => ({ default: m.AboutPage })),
);
const DebugLogPage = lazy(() =>
  import('@/pages/system/DebugLogPage').then((m) => ({ default: m.DebugLogPage })),
);
const AppearanceSettingsPage = lazy(() =>
  import('@/pages/settings/AppearanceSettingsPage').then((m) => ({
    default: m.AppearanceSettingsPage,
  })),
);
const BackupConfigPage = lazy(() =>
  import('@/pages/settings/BackupConfigPage').then((m) => ({ default: m.BackupConfigPage })),
);
const PluginGatePage = lazy(() =>
  import('@/pages/ai/PluginGatePage').then((m) => ({ default: m.PluginGatePage })),
);
const LlmChatPage = lazy(() =>
  import('@/pages/ai/LlmChatPage').then((m) => ({ default: m.LlmChatPage })),
);
const LlmConfigPage = lazy(() =>
  import('@/pages/ai/LlmConfigPage').then((m) => ({ default: m.LlmConfigPage })),
);
const TemplateManagerPage = lazy(() =>
  import('@/pages/settings/TemplateManagerPage').then((m) => ({ default: m.TemplateManagerPage })),
);
const OcrSettingsPage = lazy(() =>
  import('@/pages/settings/OcrSettingsPage').then((m) => ({ default: m.OcrSettingsPage })),
);
const GlobalAttachmentManager = lazy(() =>
  import('@/pages/settings/GlobalAttachmentManager').then((m) => ({
    default: m.GlobalAttachmentManager,
  })),
);
const VaultDirectoryPage = lazy(() =>
  import('@/pages/settings/VaultDirectoryPage').then((m) => ({ default: m.VaultDirectoryPage })),
);
const LlmStatsPage = lazy(() =>
  import('@/pages/ai/LlmStatsPage').then((m) => ({ default: m.LlmStatsPage })),
);
const HelpPage = lazy(() => import('@/pages/help/HelpPage').then((m) => ({ default: m.HelpPage })));
const ScanLocalPage = lazy(() =>
  import('@/pages/scan/ScanLocalPage').then((m) => ({ default: m.ScanLocalPage })),
);
const OcrPage = lazy(() => import('@/pages/scan/OcrPage').then((m) => ({ default: m.OcrPage })));
const HistoryPage = lazy(() =>
  import('@/pages/editor/HistoryPage').then((m) => ({ default: m.HistoryPage })),
);
const SyncPage = lazy(() => import('@/pages/sync/SyncPage').then((m) => ({ default: m.SyncPage })));
import { useAuthStore } from '@/stores/authStore';
import type { ReactNode } from 'react';

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
