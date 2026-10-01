//! RF022 实际同步页入口 + 懒加载恢复组件 + 真实 hook，仅 mock IPC/平台。
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, expect, it, vi } from 'vitest';
import i18next from 'i18next';
import { PairingPanel } from './PairingPanel';
import { setRequestSession } from '@/lib/sessionRequests';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  check: vi.fn().mockResolvedValue(undefined),
  auth: {
    isAuthenticated: true,
    currentAccount: { id: 'account', name: 'Synthetic' } as { id: string; name: string } | null,
  },
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('@/hooks/useCameraCapability', () => ({ useCameraCapability: () => 'unsupported' }));
vi.mock('@/stores/authStore', () => ({
  saveLastAccountId: vi.fn(),
  useAuthStore: Object.assign(
    (selector: (state: typeof mocks.auth) => unknown) => selector(mocks.auth),
    {
      getState: () => ({
        ...mocks.auth,
        checkHasAccount: mocks.check,
        listAccounts: vi.fn().mockResolvedValue(undefined),
      }),
    },
  ),
}));
vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  const { default: engine } = await import('i18next');
  return { ...actual, useTranslation: () => ({ t: engine.t.bind(engine), i18n: engine }) };
});
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

beforeEach(() => {
  vi.clearAllMocks();
  mocks.auth.isAuthenticated = true;
  mocks.auth.currentAccount = { id: 'account', name: 'Synthetic' };
  setRequestSession('account');
});
function renderPanel() {
  return render(
    <MemoryRouter>
      <PairingPanel
        syncEnabled={false}
        isLoading={false}
        pairPeer={null}
        isWaitingFlow={false}
        pairWaitState="idle"
        showQrOpen={false}
        scanQrOpen={false}
        onShowQrOpen={vi.fn()}
        onScanQrOpen={vi.fn()}
        onScanSync={vi.fn()}
        onTrust={vi.fn()}
        onIgnore={vi.fn()}
        onCancelWaiting={vi.fn()}
      />
    </MemoryRouter>,
  );
}
async function openManualRecovery() {
  fireEvent.click(
    screen.getByRole('button', { name: i18next.t('common:recovery_existing_receive_open') }),
  );
  // 等待真实冷加载完成，再检查表单；不把默认 1 秒查询超时当作入口行为。
  await act(async () => {
    await vi.dynamicImportSettled();
  });
  fireEvent.change(await screen.findByLabelText(i18next.t('common:recovery_receive_addr_label')), {
    target: { value: '127.0.0.1:12545' },
  });
  fireEvent.change(screen.getByLabelText(i18next.t('common:recovery_receive_pin_label')), {
    target: { value: '123456' },
  });
  fireEvent.click(screen.getByRole('button', { name: i18next.t('common:next') }));
}

it('RF022 authenticated entry remains reachable with sync disabled and exposes only existing Fresh', async () => {
  mocks.invoke.mockResolvedValue({
    operationId: 'new-operation',
    sessionGeneration: 1,
    status: 'complete',
    accountId: 'account',
    accountName: 'Synthetic',
    objectCount: 1,
    attachmentCount: 0,
    templateCount: 0,
    snapshotCount: 0,
    preferencesImported: false,
    attachmentFilesWritten: 0,
    failureStage: null,
    errorCode: null,
  });
  renderPanel();
  await openManualRecovery();
  expect(
    screen.queryByLabelText(i18next.t('common:recovery_receive_password_label')),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: i18next.t('common:recovery_overwrite_confirm') }),
  ).not.toBeInTheDocument();
  fireEvent.click(
    screen.getByRole('button', { name: i18next.t('common:recovery_existing_fresh_start') }),
  );
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith(
      'recovery_restore_existing_from_host',
      {
        accountId: 'account',
        hostAddr: '127.0.0.1:12545',
        pin: '123456',
        fingerprint: null,
        nonce: null,
      },
      expect.any(Object),
    ),
  );
  expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'recovery_restore_from_host')).toBe(false);
});

it('RF022 existing-only UI with locked account has no create or overwrite form', async () => {
  mocks.auth.isAuthenticated = false;
  mocks.auth.currentAccount = null;
  setRequestSession(null);
  renderPanel();
  await openManualRecovery();
  expect(
    screen.queryByLabelText(i18next.t('common:recovery_receive_password_label')),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: i18next.t('common:recovery_overwrite_confirm') }),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: i18next.t('common:recovery_existing_fresh_start') }),
  ).not.toBeInTheDocument();
  expect(mocks.invoke).not.toHaveBeenCalled();
});
