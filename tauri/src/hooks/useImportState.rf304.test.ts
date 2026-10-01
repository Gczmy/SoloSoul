import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import { setRequestSession } from '@/lib/sessionRequests';
import fixture from '../../src-tauri/src/commands/export_import/contracts/fixtures.json';
import { useImportState } from './useImportState';

const io = vi.hoisted(() => ({ invoke: vi.fn(), cleanup: vi.fn() }));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: io.invoke }));
vi.mock('@/lib/mobileFileTransfer', () => ({
  cleanupStagedFile: io.cleanup,
  isUriPath: () => false,
  stageImportPackage: vi.fn(),
}));
beforeEach(async () => {
  io.invoke.mockReset();
  io.cleanup.mockReset();
  setRequestSession(null);
  setRequestSession('synthetic-account');
  await i18n.changeLanguage('en-US');
  vi.spyOn(crypto, 'randomUUID').mockReturnValue('11111111-1111-4111-8111-111111111111');
});
afterEach(() => {
  cleanup();
  setRequestSession(null);
  vi.restoreAllMocks();
});

describe('RF304 actual serde fixtures through typed import flow', () => {
  it.each(['complete', 'partial', 'notCommitted'] as const)(
    'keeps %s distinct after preview and explicit empty attachment selection',
    async (status) => {
      io.invoke.mockImplementation(async (command: string) => {
        if (command === 'import_operations_list') return [];
        if (command === 'import_parse_package') return fixture.preview;
        if (command === 'import_decrypt_preview') return fixture.decrypted;
        if (command === 'import_execute_advanced') return fixture[status];
        if (command === 'import_operation_get') return fixture.operation;
        throw new Error('Unexpected IPC: ' + command);
      });
      const onSuccess = vi.fn(),
        onError = vi.fn(),
        reloadScope = vi.fn();
      const { result } = renderHook(() =>
        useImportState({
          accountId: 'synthetic-account',
          onSuccess,
          onError,
          reloadScope,
          t: i18n.t.bind(i18n),
          i18n,
        }),
      );
      await waitFor(() => expect(result.current.importOperations.loading).toBe(false));
      act(() => {
        result.current.onSetImportPath(fixture.preview.filePath);
        result.current.setImportPw('synthetic-password');
      });
      await act(async () => {
        await result.current.onPreview();
      });
      expect(result.current.importPreview).toEqual(fixture.preview);
      await act(async () => {
        await result.current.onDecrypt();
      });
      expect(result.current.decryptedPreview).toEqual(fixture.decrypted);
      act(() => result.current.onToggleImportAttachment(fixture.attachment.id));
      await act(async () => {
        await result.current.onImport();
      });
      expect(io.invoke).toHaveBeenCalledWith(
        'import_execute_advanced',
        {
          accountId: 'synthetic-account',
          req: {
            operationId: fixture.complete.operationId,
            sourcePath: fixture.preview.filePath,
            password: 'synthetic-password',
            strategy: 'skipExisting',
            selections: [{ objectId: 'synthetic-object', selected: true }],
            selectedAttachmentIds: [],
            objectStrategies: {},
            locale: 'en-US',
          },
        },
        expect.anything(),
      );
      if (status === 'complete') {
        expect(onSuccess).toHaveBeenCalledOnce();
        expect(onError).not.toHaveBeenCalled();
        expect(result.current.importPath).toBe('');
      } else {
        expect(onSuccess).not.toHaveBeenCalled();
        expect(onError).toHaveBeenCalled();
        expect(result.current.importPath).toBe(fixture.preview.filePath);
      }
      if (status === 'partial') {
        expect(result.current.importOperations.selected?.outcome).toEqual(fixture.partial);
        expect(result.current.importOperations.canResume).toBe(true);
        expect(result.current.importOperations.selected?.outcome.attachmentFilesWritten).toBe(1);
        expect(result.current.importOperations.selected?.outcome.attachmentCount).toBe(0);
      }
    },
  );
});
