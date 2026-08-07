import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, useNavigate, useLocation } from 'react-router-dom';
import { LlmConfigPage } from './LlmConfigPage';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';

vi.mock('@/components/layout/AppShell', () => ({
  AppShell: ({
    children,
    title,
    onBack,
  }: {
    children: React.ReactNode;
    title: string;
    onBack?: () => void;
  }) => (
    <div data-testid="app-shell" data-title={title}>
      {onBack && (
        <button data-testid="back-btn" onClick={onBack}>
          Back
        </button>
      )}
      {children}
    </div>
  ),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: vi.fn(),
    useLocation: vi.fn(),
  };
});

const mockProviders = [
  {
    id: 'openai',
    name: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4o',
    isEnabled: false,
    isBuiltIn: true,
    apiKey: '',
    apiType: 'openAI',
  },
  {
    id: 'ollama',
    name: 'Ollama',
    baseUrl: 'http://localhost:11434',
    model: 'llama3',
    isEnabled: false,
    isBuiltIn: true,
    apiKey: '',
    apiType: 'openAI',
  },
];

/**
 * 与后端 `llm_get_embed_models` 实际形状一致（embed_model.rs `EmbedModelWithStatus`
 * `#[serde(flatten)]` 扁平字段 + snake_case + installed）——回归「扁平数据渲染崩溃」。
 */
const mockEmbedModels = [
  {
    id: 'all-MiniLM-L6-v2',
    name: 'MiniLM',
    description: 'Lightweight embedding model',
    disk_size: '80MB',
    dimensions: 384,
    download_url: 'https://example.com/model.zip',
    checksum: 'sha256:abc123',
    installed: true,
  },
  {
    id: 'bge-small-zh',
    name: 'BGE Small',
    description: 'Chinese embedding model',
    disk_size: '120MB',
    dimensions: 512,
    download_url: 'https://example.com/bge.zip',
    checksum: 'sha256:def456',
    installed: false,
  },
];

describe('LlmConfigPage', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(navigate);
    vi.mocked(useLocation).mockReturnValue({ state: null } as ReturnType<typeof useLocation>);
    useAuthStore.setState({ currentAccount: { id: 'acc_test', name: 'Test' } });
    // 默认数据：空 provider 列表也合法（后端始终返回 Vec）
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'llm_get_providers') return mockProviders;
      if (cmd === 'llm_get_config')
        return {
          activeProviderId: '',
          aiFeaturesEnabled: { chat: false },
          includeSystemPrompt: true,
          hasAcceptedRisk: false,
          useLocalEmbedding: false,
          localEmbedModelId: null,
        };
      if (cmd === 'llm_check_embedding_available') return false;
      if (cmd === 'llm_get_embed_models') return mockEmbedModels;
      if (cmd === 'llm_get_api_key') return '';
      return undefined;
    });
  });

  it('renders all sections when data resolves (regression: blank LLM config page)', async () => {
    render(
      <MemoryRouter>
        <LlmConfigPage />
      </MemoryRouter>,
    );

    // AppShell 标题渲染
    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-title', 'settings:llm_config');

    // 数据解析完成后各区块可见
    await waitFor(() => {
      expect(screen.getByText('settings:ai_features')).toBeInTheDocument();
    });
    expect(screen.getByText('settings:ai_chat')).toBeInTheDocument();
    expect(screen.getByText('settings:ai_system_prompt_title')).toBeInTheDocument();
    expect(screen.getByText('settings:ai_service_providers')).toBeInTheDocument();
    // 云端 provider（OpenAI）应显示名称；列表渲染不崩溃
    expect(screen.getByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('Ollama')).toBeInTheDocument();
    // 未接受风险时显示风险提示条
    expect(screen.getByText('settings:ai_risk_notice')).toBeInTheDocument();
    // 本地嵌入模型列表按真实扁平形状渲染（回归：`m.info.*` 嵌套假设在扁平数据下抛 TypeError 整页卸载）
    expect(screen.getByText('MiniLM')).toBeInTheDocument();
    expect(screen.getByText('BGE Small')).toBeInTheDocument();
    // 统计入口卡片
    expect(screen.getByText('settings:llm_stats_title')).toBeInTheDocument();
  });

  it('does not crash when provider list is empty', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'llm_get_providers') return [];
      if (cmd === 'llm_get_config')
        return { activeProviderId: '', aiFeaturesEnabled: { chat: false } };
      if (cmd === 'llm_check_embedding_available') return false;
      if (cmd === 'llm_get_embed_models') return [];
      return undefined;
    });

    render(
      <MemoryRouter>
        <LlmConfigPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:ai_service_providers')).toBeInTheDocument();
    });
    // 空列表不崩溃，添加按钮仍存在
    expect(screen.getByText('settings:llm_add_custom')).toBeInTheDocument();
  });
});
