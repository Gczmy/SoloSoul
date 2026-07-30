import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
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
  getActiveHistory: () => OcrScanEntry[];
  getTrash: () => OcrScanEntry[];
  getCurrentEntry: () => OcrScanEntry | null;
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
      activeTier: 'small',
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

      getActiveHistory: () => get().scanHistory.filter((h) => !h.isDeleted),
      getTrash: () => get().scanHistory.filter((h) => h.isDeleted),
      getCurrentEntry: () => {
        const s = get();
        return s.scanHistory.find((h) => h.id === s.currentScanId) || null;
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
