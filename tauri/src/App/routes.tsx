import { Navigate } from 'react-router-dom';
import { HomePage } from '@/pages/home/HomePage';
import { SettingsPage } from '@/pages/settings/SettingsPage';
import { SecuritySettingsPage } from '@/pages/settings/SecuritySettingsPage';
import { DataManagementPage } from '@/pages/settings/DataManagementPage';
import { TrashPage } from '@/pages/settings/TrashPage';
import { ObjectWorkspacePage } from '@/pages/workspace/ObjectWorkspacePage';
import { ObjectEditorPage } from '@/pages/editor/ObjectEditorPage';
import { ExportImportPage } from '@/pages/settings/ExportImportPage';
import { SearchPage } from '@/pages/search/SearchPage';
import { OperationLogPage } from '@/pages/settings/OperationLogPage';
import { AboutPage } from '@/pages/system/AboutPage';
import { DebugLogPage } from '@/pages/system/DebugLogPage';
import { AppearanceSettingsPage } from '@/pages/settings/AppearanceSettingsPage';
import { BackupConfigPage } from '@/pages/settings/BackupConfigPage';
import { PluginGatePage } from '@/pages/ai/PluginGatePage';
import { LlmChatPage } from '@/pages/ai/LlmChatPage';
import { LlmConfigPage } from '@/pages/ai/LlmConfigPage';
import { TemplateManagerPage } from '@/pages/settings/TemplateManagerPage';
import { OcrSettingsPage } from '@/pages/settings/OcrSettingsPage';
import { GlobalAttachmentManager } from '@/pages/settings/GlobalAttachmentManager';
import { LlmStatsPage } from '@/pages/ai/LlmStatsPage';
import { HelpPage } from '@/pages/help/HelpPage';
import { ScanLocalPage } from '@/pages/scan/ScanLocalPage';
import { OcrPage } from '@/pages/scan/OcrPage';
import { HistoryPage } from '@/pages/editor/HistoryPage';
import { SyncPage } from '@/pages/sync/SyncPage';
import { useAuthStore } from '@/stores/authStore';
import { isMobilePlatform } from '@/lib/platform';

import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  if (!isAuthenticated) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

/**
 * 桌面专属路由守卫：在移动端访问时重定向到首页。
 * 使用异步 isMobilePlatform() 而非 sync 版本，避免缓存未初始化时误判。
 */
function DesktopOnlyGuard({ children }: { children: React.ReactNode }) {
  const [blocked, setBlocked] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    isMobilePlatform().then((mobile) => {
      if (!cancelled) setBlocked(mobile);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (blocked === null) return null;
  if (blocked) return <Navigate to="/" replace />;
  return <>{children}</>;
}

/** 将元素包装为桌面专属路由，保持路由表可读性 */
function desktopOnly(element: ReactNode) {
  return <DesktopOnlyGuard>{element}</DesktopOnlyGuard>;
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
  { path: '/settings/export-import', element: <ExportImportPage /> },
  { path: '/settings/data', element: <DataManagementPage /> },
  { path: '/settings/trash', element: <TrashPage /> },
  { path: '/settings/operation-log', element: <OperationLogPage /> },
  { path: '/settings/backup', element: <BackupConfigPage /> },
  { path: '/about', element: <AboutPage /> },
  { path: '/debug-log', element: <DebugLogPage /> },
  { path: '/plugins', element: desktopOnly(<PluginGatePage />) },
  { path: '/settings/templates', element: <TemplateManagerPage /> },
  { path: '/settings/attachments', element: <GlobalAttachmentManager /> },
  { path: '/settings/ocr', element: desktopOnly(<OcrSettingsPage />) },
  { path: '/settings/llm', element: <LlmConfigPage /> },
  { path: '/settings/llm/stats', element: <LlmStatsPage /> },
  { path: '/llm-chat', element: <LlmChatPage /> },
  { path: '/local-import', element: <ScanLocalPage /> },
  { path: '/history', element: <HistoryPage /> },
  { path: '/ocr', element: desktopOnly(<OcrPage />) },
  { path: '/sync', element: desktopOnly(<SyncPage />) },
  { path: '/help', element: <HelpPage /> },
  { path: '/workspace', element: <ObjectWorkspacePage /> },
  { path: '/editor', element: <ObjectEditorPage /> },
  { path: '/editor/:objectId', element: <ObjectEditorPage /> },
  { path: '/workspace/custom/:pageId', element: <ObjectWorkspacePage /> },
];
