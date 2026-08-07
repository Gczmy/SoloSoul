import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { invokeCommand as invoke } from '@/lib/ipcClient';
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

interface OcrScanState {
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

export const useOcrScanStore = create<OcrScanState>()(
  persist(
    (set, get) => ({
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
        const state = get();
        const fileName = filePath.split(/[/\\]/).pop() || 'unknown';
        const id = `scan_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
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

        set({
          isScanning: true,
          currentScanId: id,
          lastScanError: null,
          scanHistory: [entry, ...state.scanHistory].slice(0, HISTORY_LIMIT),
        });

        try {
          if (state.scanMode === 'mrz') {
            const res = await invoke<MrzResult | null>('ocr_scan_mrz', { filePath: filePath });
            if (res) {
              set((s) => ({
                isScanning: false,
                scanHistory: s.scanHistory.map((h) => (h.id === id ? { ...h, mrzResult: res } : h)),
                lastScanError: null,
              }));
            } else {
              // 未检测到 MRZ 时自动 fallback 到通用 OCR
              const fallback = await invoke<OcrResult>('ocr_scan_image', { filePath: filePath });
              set((s) => ({
                isScanning: false,
                scanHistory: s.scanHistory.map((h) =>
                  h.id === id ? { ...h, result: fallback } : h,
                ),
                lastScanError: null,
              }));
            }
          } else {
            const res = await invoke<OcrResult>('ocr_scan_image', { filePath: filePath });
            set((s) => ({
              isScanning: false,
              scanHistory: s.scanHistory.map((h) => (h.id === id ? { ...h, result: res } : h)),
              lastScanError: null,
            }));
          }
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          set((s) => ({
            isScanning: false,
            lastScanError: msg,
            scanHistory: s.scanHistory.map((h) => (h.id === id ? { ...h, error: msg } : h)),
          }));
        }
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
      clearOnVaultLock: () =>
        set({
          scanHistory: [],
          currentScanId: null,
          lastScanError: null,
          isScanning: false,
          isCardOpen: false,
        }),
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
