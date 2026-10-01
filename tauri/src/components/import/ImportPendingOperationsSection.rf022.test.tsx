import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import { initReactI18next } from 'react-i18next';
import enSettings from '@/locales/en-US/settings.json';
import enCommon from '@/locales/en-US/common.json';
import { setRequestSession } from '@/lib/sessionRequests';
import { useImportState } from '@/hooks/useImportState';
import type { ImportOperationSummary, ImportResult } from '@/types/exportImport';
import { ImportPendingOperationsSection } from './ImportPendingOperationsSection';

const io = vi.hoisted(() => ({ invoke: vi.fn(), choose: vi.fn() }));
vi.unmock('react-i18next');
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: io.invoke }));
vi.mock('@/lib/dialog', () => ({ openWithPause: io.choose }));
const ID = '11111111-1111-4111-8111-111111111111';
const outcome: ImportResult = {
  operationId: ID,
  sessionGeneration: 7,
  status: 'partial',
  objectCount: 2,
  attachmentCount: 0,
  templateCount: 0,
  snapshotCount: 2,
  preferencesImported: false,
  attachmentFilesWritten: 0,
  failureStage: 'attachments',
  errorCode: 'IMPORT_FAILED',
};
let pending: ImportOperationSummary;
const onError = vi.fn();
const onSuccess = vi.fn();
function Harness() {
  const hook = useImportState({
    accountId: 'account-a',
    onError,
    onSuccess,
    reloadScope: vi.fn(),
    t: i18n.t.bind(i18n),
    i18n,
  });
  return <ImportPendingOperationsSection state={hook.importOperations} />;
}
async function select() {
  const row = await screen.findByRole('button', { name: 'Resume import' });
  fireEvent.click(row);
  await waitFor(() =>
    expect(io.invoke.mock.calls.some(([name]) => name === 'import_operation_get')).toBe(true),
  );
  await screen.findByText(
    'Resuming uses the original selection and strategy. Start a new import to change them.',
  );
}
const resume = () => screen.getAllByRole('button', { name: 'Resume import' }).at(-1)!;
beforeAll(async () => {
  await i18n
    .use(initReactI18next)
    .init({
      lng: 'en-US',
      fallbackLng: 'en-US',
      ns: ['settings', 'common'],
      defaultNS: 'settings',
      resources: { 'en-US': { settings: enSettings, common: enCommon } },
      interpolation: { escapeValue: false },
    });
});
beforeEach(async () => {
  io.invoke.mockReset();
  io.choose.mockReset();
  onError.mockReset();
  onSuccess.mockReset();
  setRequestSession(null);
  setRequestSession('account-a');
  await i18n.changeLanguage('en-US');
  pending = {
    operationId: ID,
    phase: 'attachments',
    sourceKind: 'manual',
    sourceName: 'original.solosoul',
    createdAt: '2026-09-30',
    updatedAt: '2026-09-30',
    sourceRequired: false,
    passwordRequired: false,
    outcome,
  };
  io.choose.mockResolvedValue(null);
  io.invoke.mockImplementation(async (command: string) => {
    if (command === 'import_operations_list') return [pending];
    if (command === 'import_operation_get') return pending;
    if (command === 'import_operation_resume')
      return {
        ...outcome,
        status: 'complete',
        attachmentCount: 2,
        attachmentFilesWritten: 2,
        failureStage: null,
        errorCode: null,
      };
    throw new Error('Unexpected IPC: ' + command);
  });
});
afterEach(() => {
  cleanup();
  setRequestSession(null);
});

describe('RF022 independent pending import UI with real hook', () => {
  it('ready pending task can continue without a decrypted preview or chosen objects', async () => {
    render(<Harness />);
    await select();
    expect(resume()).toBeEnabled();
    fireEvent.click(resume());
    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
    expect(io.invoke.mock.calls.find(([name]) => name === 'import_operation_resume')?.[1]).toEqual({
      accountId: 'account-a',
      operationId: ID,
      password: null,
      sourcePath: null,
    });
    expect(
      io.invoke.mock.calls.some(([name]) => /preview|decrypt|execute_advanced/.test(name)),
    ).toBe(false);
  });
  it('asks for the original package password only when required and clears it when continuing later', async () => {
    pending = { ...pending, passwordRequired: true };
    render(<Harness />);
    await select();
    expect(resume()).toBeDisabled();
    const password = screen.getByLabelText('Enter the original package password');
    fireEvent.change(password, { target: { value: 'original-package-password' } });
    await waitFor(() => expect(resume()).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Continue later' }));
    await waitFor(() =>
      expect(screen.queryByLabelText('Enter the original package password')).toBeNull(),
    );
    expect(io.invoke.mock.calls.some(([name]) => /resume$|delete|cancel/.test(name))).toBe(false);
  });
  it('requires an original-package reselect without turning a canceled picker into Fresh', async () => {
    pending = { ...pending, sourceRequired: true };
    render(<Harness />);
    await select();
    expect(resume()).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Select File' }));
    await waitFor(() => expect(io.choose).toHaveBeenCalledOnce());
    expect(resume()).toBeDisabled();
    io.choose.mockResolvedValue('C:/original-again.solosoul');
    fireEvent.click(screen.getByRole('button', { name: 'Select File' }));
    await waitFor(() => expect(resume()).toBeEnabled());
    fireEvent.click(resume());
    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
    expect(io.invoke.mock.calls.find(([name]) => name === 'import_operation_resume')?.[1]).toEqual({
      accountId: 'account-a',
      operationId: ID,
      password: null,
      sourcePath: 'C:/original-again.solosoul',
    });
    expect(io.invoke.mock.calls.some(([name]) => name === 'import_execute_advanced')).toBe(false);
  });
  it('an accepted ready recovery task has no random transfer password or new-account action', async () => {
    pending = { ...pending, sourceKind: 'recovery' };
    render(<Harness />);
    await select();
    expect(screen.queryByLabelText('Enter the original package password')).toBeNull();
    expect(resume()).toBeEnabled();
    fireEvent.click(resume());
    await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
    expect(io.invoke.mock.calls.some(([name]) => /restore|account.*create/.test(name))).toBe(false);
  });
});
