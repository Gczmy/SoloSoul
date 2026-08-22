import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, useParams, useSearchParams, useNavigate } from 'react-router-dom';
import { ObjectWorkspacePage } from './ObjectWorkspacePage';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { UserTemplate } from '@/types/template';

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="page-shell" data-title={title}>
      {children}
    </div>
  ),
}));

// 稳定 t（setup.ts 的 mock 每次渲染返回新函数，会让依赖 t 的 effect 无限重跑）
const { stableT } = vi.hoisted(() => ({
  stableT: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: stableT,
    i18n: { language: 'zh', changeLanguage: vi.fn(() => Promise.resolve()) },
  }),
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useParams: vi.fn(),
    useSearchParams: vi.fn(),
    useNavigate: vi.fn(),
  };
});

// 与真实身份模板一致：dateOfBirth 为 internal 敏感度（卡片应掩码显示占位符）
const identityTemplate: UserTemplate = {
  id: 'identity',
  accountId: 'acc1',
  name: '身份信息',
  iconId: 'identity',
  category: 'identity',
  createdAt: '2026-08-22T00:00:00Z',
  updatedAt: '2026-08-22T00:00:00Z',
  properties: [
    { id: 'fullName', name: '姓名', type: 'text', sensitivityLevel: 'public' },
    { id: 'dateOfBirth', name: '出生日期', type: 'date', sensitivityLevel: 'internal' },
  ],
};

describe('ObjectWorkspacePage card field display', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(navigate);
    vi.mocked(useParams).mockReturnValue({});
    vi.mocked(useSearchParams).mockReturnValue([new URLSearchParams('section=identity'), vi.fn()]);
    useAuthStore.setState({
      currentAccount: { id: 'acc1', name: 'Acc' },
      isAuthenticated: true,
    });
    useTemplateStore.setState({
      templates: [identityTemplate],
      loadTemplates: vi.fn().mockResolvedValue(undefined),
    });
    useSettingsStore.setState((s) => ({
      settings: { ...s.settings, customPages: [] },
      removeCustomPage: vi.fn(),
    }));
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'template_list') return [identityTemplate];
      if (cmd === 'template_hash_map') return { identity: 'abc123' };
      if (cmd === 'object_list') {
        // 真实 object_list 截断结果：含 __fields，dateOfBirth 值完整保留
        return [
          {
            id: 'obj1',
            name: '张三',
            typeId: 'identity',
            sensitivityLevel: 'internal',
            createdAt: '2026-08-22T00:00:00Z',
            updatedAt: '2026-08-22T00:00:00Z',
            templateId: 'identity',
            templateType: 'user',
            properties: {
              fullName: '张三',
              dateOfBirth: '2024-12-31',
              __fields: {
                fullName: { name: '姓名', type: 'text' },
                dateOfBirth: { name: '出生日期', type: 'date' },
              },
              __templateName: '身份信息',
            },
            propertyLabels: { fullName: 'public', dateOfBirth: 'internal' },
            tags: [],
          },
        ];
      }
      if (cmd === 'snapshot_count_batch') return {};
      if (cmd === 'attachment_count_batch') return {};
      if (cmd === 'biometric_check_availability') return { available: false, configured: false };
      if (cmd === 'vault_list_accounts') return [];
      return undefined;
    });
  });

  it('shows the date field chip on the card; internal sensitivity value is masked', async () => {
    render(
      <MemoryRouter>
        <ObjectWorkspacePage />
      </MemoryRouter>,
    );

    // 等对象列表加载完成、卡片出现（对象名 + 姓名 chip 均为「张三」）
    await waitFor(() => {
      expect(screen.getAllByText('张三').length).toBeGreaterThan(0);
    });

    // 日期字段 chip 必须渲染（label 出现）——值按 internal 敏感度掩码
    expect(screen.getByText('出生日期')).toBeInTheDocument();
    // internal 字段值掩码为占位圆点（P036 设计），而不是空白或消失
    expect(screen.getAllByText('••••••••').length).toBeGreaterThan(0);
    // public 字段（姓名）原样显示
    expect(screen.getAllByText('张三').length).toBeGreaterThan(1);
  });
});
