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
