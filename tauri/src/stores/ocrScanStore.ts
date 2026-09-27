import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import {
  createOcrScanOperation,
  type OcrJobState,
  type OcrScanOperation,
} from '@/lib/ocrScanOperation';
import { useAuthStore } from '@/stores/authStore';
import { isMacOSSync } from '@/lib/platform';
import type { OcrResult, MrzResult } from '@/lib/ipc';

export interface OcrScanEntry {
  id: string;
  timestamp: number;
  filePath: string;
  fileName: string;
  mode: 'general' | 'mrz';
  result: OcrResult | null;
  mrzResult: MrzResult | null;
  isDeleted: boolean;
  deletedAt?: number;
  error?: string;
}

export interface OcrScanCompletion {
  operationId: string;
  status: 'completed' | 'cancelled' | 'failed';
  error: string | null;
}

interface OcrScanState {
  scanState: OcrJobState | null;
  lastCompletion: OcrScanCompletion | null;
  cancelScan: () => void;
  claimCompletion: (operationId: string) => OcrScanCompletion | null;
  isCardOpen: boolean;
  scanMode: 'general' | 'mrz';
  scanHistory: OcrScanEntry[];
  currentScanId: string | null;
  isScanning: boolean;
  activeTier: string;
  lastScanError: string | null;

  setCardOpen: (open: boolean) => void;
  setScanMode: (mode: 'general' | 'mrz') => void;
  setActiveTier: (tier: string) => void;

  performScan: (filePath: string) => Promise<void>;
  softDeleteEntry: (id: string) => void;
  restoreEntry: (id: string) => void;
  permanentlyDeleteEntry: (id: string) => void;
  clearTrash: () => void;

  /** P230: Vault 锁定/退出时清空扫描结果明文（含 MRZ 证件号），仅保留持久化元数据。 */
  clearOnVaultLock: () => void;
}

const HISTORY_LIMIT = 50;
const scanRequests = createSessionRequests();
let activeOperation: OcrScanOperation | null = null;
let completionClaim: { operationId: string; isCurrent: () => boolean; claimed: boolean } | null =
  null;

export const useOcrScanStore = create<OcrScanState>()(
  persist(
    (set, get) => ({
      scanState: null,
      lastCompletion: null,
      cancelScan: () => activeOperation?.cancel(),
      claimCompletion: (operationId) => {
        const completion = get().lastCompletion;
        if (
          !completion ||
          completion.operationId !== operationId ||
          completionClaim?.operationId !== operationId ||
          completionClaim.claimed ||
          !completionClaim.isCurrent()
        )
          return null;
        completionClaim.claimed = true;
        return completion;
      },
      isCardOpen: false,
      scanMode: 'general',
      scanHistory: [],
      currentScanId: null,
      isScanning: false,
      // P133: macOS 默认 Vision 引擎（后端加载前兜底；权威值以 ocr_get_active_tier 为准）。
      activeTier: isMacOSSync() ? 'vision' : 'small',
      lastScanError: null,

      setCardOpen: (open) => set({ isCardOpen: open }),
      setScanMode: (mode) => set({ scanMode: mode }),
      setActiveTier: (tier) => set({ activeTier: tier }),

      performScan: async (filePath: string) => {
        activeOperation?.dispose();
        const accountId = useAuthStore.getState().currentAccount?.id;
        const ticket = scanRequests.begin('scan', accountId);
        const state = get();
        const fileName = filePath.split(/[/\\]/).pop() || 'unknown';
        const id = crypto.randomUUID();
        const entry: OcrScanEntry = {
          id,
          timestamp: Date.now(),
          filePath,
          fileName,
          mode: state.scanMode,
          result: null,
          mrzResult: null,
          isDeleted: false,
        };
        completionClaim = null;
        set({
          isScanning: true,
          scanState: 'queued',
          lastCompletion: null,
          currentScanId: id,
          lastScanError: null,
          scanHistory: [entry, ...state.scanHistory].slice(0, HISTORY_LIMIT),
        });
        const operation = createOcrScanOperation({
          accountId,
          isCurrent: ticket.isCurrent,
          onState: (scanState) => {
            if (ticket.isCurrent()) set({ scanState });
          },
        });
        activeOperation = operation;
        const outcome = await operation.run(filePath, state.scanMode);
        if (!ticket.isCurrent() || !operation.isCurrent() || activeOperation !== operation) return;
        activeOperation = null;
        const error = outcome.status === 'failed' ? outcome.error : null;
        const lastCompletion =
          outcome.status === 'stale' ? null : { operationId: id, status: outcome.status, error };
        completionClaim = lastCompletion
          ? { operationId: id, isCurrent: ticket.isCurrent, claimed: false }
          : null;
        set((s) => ({
          isScanning: false,
          scanState: outcome.status,
          lastScanError: error,
          lastCompletion,
          scanHistory: s.scanHistory.map((item) =>
            item.id !== id
              ? item
              : {
                  ...item,
                  result: outcome.status === 'completed' ? outcome.result : null,
                  mrzResult: outcome.status === 'completed' ? outcome.mrzResult : null,
                  ...(error ? { error } : {}),
                },
          ),
        }));
      },
      softDeleteEntry: (id) =>
        set((s) => ({
          scanHistory: s.scanHistory.map((h) =>
            h.id === id ? { ...h, isDeleted: true, deletedAt: Date.now() } : h,
          ),
        })),

      restoreEntry: (id) =>
        set((s) => ({
          scanHistory: s.scanHistory.map((h) =>
            h.id === id ? { ...h, isDeleted: false, deletedAt: undefined } : h,
          ),
        })),

      permanentlyDeleteEntry: (id) =>
        set((s) => ({
          scanHistory: s.scanHistory.filter((h) => h.id !== id),
        })),

      clearTrash: () =>
        set((s) => ({
          scanHistory: s.scanHistory.filter((h) => !h.isDeleted),
        })),

      // P230: 锁定/退出后清空含解密明文的内存态（result/mrzResult/filePath 均含敏感内容）。
      // 只读 UI 偏好（activeTier/scanMode）不受影响；persist partialize 本就不持久化结果。
      clearOnVaultLock: () => {
        activeOperation?.dispose();
        activeOperation = null;
        scanRequests.invalidate();
        completionClaim = null;
        set({
          scanHistory: [],
          currentScanId: null,
          lastScanError: null,
          lastCompletion: null,
          scanState: null,
          isScanning: false,
          isCardOpen: false,
        });
      },
    }),
    {
      name: 'solosoul-ocr-scan-history',
      storage: createJSONStorage(() => localStorage),
      // 仅持久化非敏感元数据（activeTier / scanMode），不持久化扫描结果（result / mrzResult / filePath）
      partialize: (state) => ({
        activeTier: state.activeTier,
        scanMode: state.scanMode,
      }),
    },
  ),
);

// 即使页面未挂载，认证状态变化也使悬停 OCR 操作失效。
onRequestSessionChange(() => useOcrScanStore.getState().clearOnVaultLock());
