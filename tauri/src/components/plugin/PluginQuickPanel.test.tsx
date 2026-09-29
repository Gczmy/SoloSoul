import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import type { MarketPluginInfo, PluginManifest } from '@/lib/generated/ipcContracts';
import type { RunningPlugin } from '@/stores/pluginStore';
import { marketPlugin, pluginManifest, registryEntry } from '@/test/pluginFixtures';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import { WATERMARK_PLUGIN_ID } from '@/lib/plugin';
import { PluginQuickPanel } from './PluginQuickPanel';

const mocks = vi.hoisted(() => ({
  state: {
    marketPlugins: [] as MarketPluginInfo[],
    installedPlugins: [] as PluginManifest[],
    installingPlugins: {},
    runningPlugins: {} as Record<string, RunningPlugin>,
    error: null,
    isLoadingMarket: false,
    isLoadingInstalled: false,
    loadMarket: vi.fn(),
    loadInstalled: vi.fn(),
    installPlugin: vi.fn(),
    cancelInstall: vi.fn(),
    uninstallPlugin: vi.fn(),
    runPlugin: vi.fn(),
    stopPlugin: vi.fn(),
    clearPluginOutput: vi.fn(),
    refreshRegistry: vi.fn(),
  },
  showToast: vi.fn(),
}));

vi.mock('@/stores/pluginStore', () => ({
  usePluginStore: <T,>(selector: (state: typeof mocks.state) => T) => selector(mocks.state),
}));
vi.mock('@/stores/uiStore', () => ({
  useUiStore: { getState: () => ({ showToast: mocks.showToast }) },
}));
vi.mock('@/lib/utils', async () => ({
  ...(await vi.importActual<typeof import('@/lib/utils')>('@/lib/utils')),
  isDevOrDebug: () => true,
}));
vi.mock('./WatermarkPluginConfig', () => ({
  WatermarkPluginConfig: ({
    onParamsChange,
  }: {
    onParamsChange: (params: Record<string, string>) => void;
  }) => (
    <>
      <button onClick={() => onParamsChange({ selectedAttachments: '[]' })}>Empty selection</button>
      <button onClick={() => onParamsChange({ selectedAttachments: '["attachment"]' })}>
        Select attachment
      </button>
    </>
  ),
}));
vi.mock('./shared/PluginLogSection', () => ({
  PluginLogSection: ({ onStop, onClear }: { onStop: () => void; onClear: () => void }) => (
    <>
      <button onClick={onStop}>Stop run</button>
      <button onClick={onClear}>Clear output</button>
    </>
  ),
}));
vi.mock('./shared/PluginResultSection', () => ({ PluginResultSection: () => null }));

function renderPanel(onClose = vi.fn()) {
  return render(
    <MemoryRouter>
      <PluginQuickPanel position={{ top: 80 }} onClose={onClose} />
    </MemoryRouter>,
  );
}

function installedPlugin(pluginId: string, name: string): MarketPluginInfo {
  return marketPlugin({
    pluginId,
    installedVersion: '1.0.0',
    registryEntry: registryEntry({ name }),
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  usePluginQuickStore.setState({ activeTab: 'all' });
  mocks.state.marketPlugins = [];
  mocks.state.installedPlugins = [];
  mocks.state.runningPlugins = {};
  mocks.state.error = null;
  mocks.state.isLoadingMarket = false;
  mocks.state.isLoadingInstalled = false;
});

describe('PluginQuickPanel', () => {
  it('保留卸载确认框，直到确认后才卸载并允许关闭底层面板', async () => {
    mocks.state.marketPlugins = [installedPlugin('plugin-a', 'Plugin A')];
    mocks.state.installedPlugins = [pluginManifest({ id: 'plugin-a' })];
    const onClose = vi.fn();
    renderPanel(onClose);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    fireEvent.click(screen.getByRole('button', { name: 'Uninstall' }));
    expect(screen.getByText('Uninstall Plugin')).toBeVisible();
    fireEvent.mouseDown(screen.getByText('Uninstall Plugin'));
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText('Uninstall Plugin')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
    expect(mocks.state.uninstallPlugin).toHaveBeenCalledOnce();
    expect(mocks.state.uninstallPlugin).toHaveBeenCalledWith('plugin-a');
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('水印附件为空时阻止运行，选择附件后传递配置', () => {
    mocks.state.marketPlugins = [installedPlugin(WATERMARK_PLUGIN_ID, 'Watermark')];
    renderPanel();

    fireEvent.click(screen.getByRole('button', { name: 'Empty selection' }));
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    expect(mocks.state.runPlugin).not.toHaveBeenCalled();
    expect(mocks.showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'warning' }));

    fireEvent.click(screen.getByRole('button', { name: 'Select attachment' }));
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    expect(mocks.state.runPlugin).toHaveBeenCalledWith(WATERMARK_PLUGIN_ID, 'Watermark', {
      selectedAttachments: '["attachment"]',
    });
  });

  it('运行中标签只显示活跃插件，停止与清除操作送往正确插件', () => {
    mocks.state.marketPlugins = [
      installedPlugin('plugin-a', 'Plugin A'),
      installedPlugin('plugin-b', 'Plugin B'),
    ];
    const running = {
      pluginId: 'plugin-b',
      pluginName: 'Plugin B',
      startTime: 1,
      logs: [],
      results: [],
      consentRequests: [],
      dialogRequests: [],
      completed: false,
    } satisfies RunningPlugin;
    mocks.state.runningPlugins = { 'plugin-b': running };
    renderPanel();

    act(() => usePluginQuickStore.getState().setActiveTab('running'));
    expect(screen.queryByText('Plugin A')).not.toBeInTheDocument();
    expect(screen.getByText('Plugin B')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Stop run' }));
    fireEvent.click(screen.getByRole('button', { name: 'Clear output' }));
    expect(mocks.state.stopPlugin).toHaveBeenCalledWith('plugin-b');
    expect(mocks.state.clearPluginOutput).toHaveBeenCalledWith('plugin-b');
  });
});
