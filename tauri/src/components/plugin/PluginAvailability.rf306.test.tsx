import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import type { MarketPluginInfo, PluginManifest } from '@/lib/generated/ipcContracts';
import { marketPlugin, registryEntry } from '@/test/pluginFixtures';
import { PluginCard } from './PluginCard';
import { PluginQuickPanel } from './PluginQuickPanel';

const mocks = vi.hoisted(() => ({
  state: {
    marketPlugins: [] as MarketPluginInfo[],
    installedPlugins: [] as PluginManifest[],
    installingPlugins: {},
    runningPlugins: {},
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
  quick: { activeTab: 'all' as const, setActiveTab: vi.fn() },
  onInstall: vi.fn(),
  onUpdate: vi.fn(),
  onUninstall: vi.fn(),
  onRun: vi.fn(),
  onClose: vi.fn(),
}));

vi.mock('@/stores/pluginStore', () => ({
  usePluginStore: <T,>(selector: (state: typeof mocks.state) => T) => selector(mocks.state),
}));
vi.mock('@/stores/pluginQuickStore', () => ({
  usePluginQuickStore: <T,>(selector: (state: typeof mocks.quick) => T) => selector(mocks.quick),
}));
vi.mock('@/hooks/useConfirm', () => ({
  useConfirm: () => ({ requestConfirm: vi.fn(), dialog: null, isOpen: false }),
}));
vi.mock('@/lib/plugin', () => ({
  hasUsableWatermarkSelection: () => true,
  WATERMARK_PLUGIN_ID: 'com.solosoul.official.watermark',
}));
vi.mock('@/lib/utils', async () => ({
  ...(await vi.importActual<typeof import('@/lib/utils')>('@/lib/utils')),
  isDevOrDebug: () => true,
}));
vi.mock('./shared/PluginLogSection', () => ({ PluginLogSection: () => null }));
vi.mock('./shared/PluginResultSection', () => ({ PluginResultSection: () => null }));
vi.mock('./WatermarkPluginConfig', () => ({ WatermarkPluginConfig: () => null }));

beforeEach(() => {
  vi.clearAllMocks();
  mocks.state.marketPlugins = [];
  mocks.state.installedPlugins = [];
});

function card(info: MarketPluginInfo) {
  return (
    <PluginCard
      info={info}
      isRunning={false}
      onInstall={mocks.onInstall}
      onUpdate={mocks.onUpdate}
      onUninstall={mocks.onUninstall}
      onRun={mocks.onRun}
    />
  );
}

function quickPanel() {
  return (
    <MemoryRouter>
      <PluginQuickPanel position={{ top: 80 }} onClose={mocks.onClose} />
    </MemoryRouter>
  );
}

describe('RF306 PluginCard version availability', () => {
  it.each([null, '', '   '])(
    'does not install or update an unavailable version %j',
    (latestVersion) => {
      const info = marketPlugin({ registryEntry: registryEntry({ latestVersion, author: null }) });
      const view = render(card(info));
      expect(screen.getByText('Version unavailable')).toBeVisible();
      const install = screen.getByRole('button', { name: 'Install' });
      expect(install).toBeDisabled();
      fireEvent.click(install);
      expect(mocks.onInstall).not.toHaveBeenCalled();

      view.rerender(card({ ...info, installedVersion: '1.0.0', hasUpdate: true }));
      const update = screen.getByRole('button', { name: 'Update' });
      expect(update).toBeDisabled();
      fireEvent.click(update);
      expect(mocks.onUpdate).not.toHaveBeenCalled();
      const run = screen.getByRole('button', { name: 'Run' });
      expect(run).toBeEnabled();
      fireEvent.click(run);
      expect(mocks.onRun).toHaveBeenCalledOnce();
    },
  );

  it('keeps valid versions installable and updateable', () => {
    const info = marketPlugin({ registryEntry: registryEntry({ latestVersion: '2.0.0' }) });
    const view = render(card(info));
    expect(screen.getByText('v2.0.0')).toBeVisible();
    const install = screen.getByRole('button', { name: 'Install' });
    expect(install).toBeEnabled();
    fireEvent.click(install);
    expect(mocks.onInstall).toHaveBeenCalledOnce();

    view.rerender(card({ ...info, installedVersion: '1.0.0', hasUpdate: true }));
    const update = screen.getByRole('button', { name: 'Update' });
    expect(update).toBeEnabled();
    fireEvent.click(update);
    expect(mocks.onUpdate).toHaveBeenCalledOnce();
  });
});

describe('RF306 PluginQuickPanel version availability', () => {
  it.each([null, '', '   '])(
    'does not dispatch an unavailable version %j or an empty author separator',
    (latestVersion) => {
      mocks.state.marketPlugins = [
        marketPlugin({
          registryEntry: registryEntry({ latestVersion, author: null }),
        }),
      ];
      const view = render(quickPanel());
      expect(screen.getByRole('dialog', { name: 'Plugins' })).toBeVisible();
      expect(screen.getByText('Version unavailable')).toHaveTextContent(/^Version unavailable$/);
      const install = screen.getByRole('button', { name: 'Install' });
      expect(install).toBeDisabled();
      fireEvent.click(install);
      expect(mocks.state.installPlugin).not.toHaveBeenCalled();

      mocks.state.marketPlugins = [
        marketPlugin({
          registryEntry: registryEntry({ latestVersion: '2.0.0', author: null }),
        }),
      ];
      view.rerender(quickPanel());
      expect(screen.getByText('v2.0.0')).toHaveTextContent(/^v2\.0\.0$/);
      const ready = screen.getByRole('button', { name: 'Install' });
      expect(ready).toBeEnabled();
      fireEvent.click(ready);
      expect(mocks.state.installPlugin).toHaveBeenCalledTimes(1);
      expect(mocks.state.installPlugin).toHaveBeenCalledWith('synthetic-plugin', '2.0.0');
    },
  );

  it('preserves author and version display while dispatching the exact available version', () => {
    mocks.state.marketPlugins = [
      marketPlugin({
        registryEntry: registryEntry({ latestVersion: '2.3.4', author: 'Synthetic author' }),
      }),
    ];
    render(quickPanel());
    expect(screen.getByText('Synthetic author · v2.3.4')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Install' }));
    expect(mocks.state.installPlugin).toHaveBeenCalledTimes(1);
    expect(mocks.state.installPlugin).toHaveBeenCalledWith('synthetic-plugin', '2.3.4');
  });
});
