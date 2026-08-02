import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// 模拟 invoke（替代旧的 commands 对象）
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// 模拟 localStorage（用于 zustand persist）
function createLocalStorageMock() {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
}

describe('ocrScanStore', () => {
  let storage: ReturnType<typeof createLocalStorageMock>;

  beforeEach(() => {
    storage = createLocalStorageMock();
    vi.stubGlobal('localStorage', storage);
    mockInvoke.mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  describe('basic state', () => {
    it('初始状态正确', async () => {
      const { useOcrScanStore } = await import('./ocrScanStore');
      const state = useOcrScanStore.getState();
      expect(state.isCardOpen).toBe(false);
      expect(state.scanMode).toBe('general');
      expect(state.isScanning).toBe(false);
      expect(state.activeTier).toBe('small');
      expect(state.scanHistory).toEqual([]);
    });

    it('setCardOpen / setScanMode / setActiveTier', async () => {
      const { useOcrScanStore } = await import('./ocrScanStore');
      useOcrScanStore.getState().setCardOpen(true);
      expect(useOcrScanStore.getState().isCardOpen).toBe(true);
      useOcrScanStore.getState().setScanMode('mrz');
      expect(useOcrScanStore.getState().scanMode).toBe('mrz');
      useOcrScanStore.getState().setActiveTier('tiny');
      expect(useOcrScanStore.getState().activeTier).toBe('tiny');
    });
  });

  describe('performScan', () => {
    it('通用模式扫描成功', async () => {
      const result = { text: 'Hello', confidence: 0.95, boxes: [] };
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_image') return result;
        return undefined;
      });

      const { useOcrScanStore } = await import('./ocrScanStore');
      await useOcrScanStore.getState().performScan('/img.png');

      const state = useOcrScanStore.getState();
      expect(state.isScanning).toBe(false);
      expect(state.scanHistory).toHaveLength(1);
      expect(state.scanHistory[0].fileName).toBe('img.png');
      expect(state.scanHistory[0].mode).toBe('general');
      expect(state.scanHistory[0].result).toEqual(result);
      expect(state.lastScanError).toBeNull();
    });

    it('MRZ 模式扫描成功', async () => {
      const mrzResult = {
        documentType: 'P',
        documentNumber: 'AB123',
        rawLines: [],
        confidence: 0.95,
        checksumValid: true,
      };
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_mrz') return mrzResult;
        return undefined;
      });

      const { useOcrScanStore } = await import('./ocrScanStore');
      useOcrScanStore.getState().setScanMode('mrz');
      await useOcrScanStore.getState().performScan('/passport.png');

      const state = useOcrScanStore.getState();
      expect(state.isScanning).toBe(false);
      expect(state.scanHistory[0].mode).toBe('mrz');
      expect(state.scanHistory[0].mrzResult).toEqual(mrzResult);
      expect(state.scanHistory[0].result).toBeNull();
    });

    it('MRZ 未检测到时 fallback 到通用 OCR', async () => {
      const fallback = { text: 'Fallback', confidence: 0.8, boxes: [] };
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_mrz') return null;
        if (cmd === 'ocr_scan_image') return fallback;
        return undefined;
      });

      const { useOcrScanStore } = await import('./ocrScanStore');
      useOcrScanStore.getState().setScanMode('mrz');
      await useOcrScanStore.getState().performScan('/no-mrz.png');

      const state = useOcrScanStore.getState();
      expect(mockInvoke).toHaveBeenCalledWith('ocr_scan_image', expect.anything()); // fallback 被调用
      expect(state.scanHistory[0].result).toEqual(fallback); // 存储在 result 而非 mrzResult
    });

    it('扫描失败时设置错误信息', async () => {
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_image') throw new Error('OCR engine error');
        return undefined;
      });

      const { useOcrScanStore } = await import('./ocrScanStore');
      await useOcrScanStore.getState().performScan('/bad.png');

      const state = useOcrScanStore.getState();
      expect(state.isScanning).toBe(false);
      expect(state.lastScanError).toBe('OCR engine error');
      expect(state.scanHistory[0].error).toBe('OCR engine error');
    });

    it('历史记录不超过 50 条', async () => {
      const result = { text: 'X', confidence: 0.9, boxes: [] };
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_image') return result;
        return undefined;
      });

      const { useOcrScanStore } = await import('./ocrScanStore');
      // 先填满 50 条
      for (let i = 0; i < 50; i++) {
        useOcrScanStore.getState().performScan(`/img${i}.png`);
        await vi.waitFor(() => {
          expect(useOcrScanStore.getState().isScanning).toBe(false);
        });
      }
      // 再多加一条 — 应仍为 50 条
      await useOcrScanStore.getState().performScan('/img-extra.png');
      expect(useOcrScanStore.getState().scanHistory).toHaveLength(50);
    });
  });

  describe('trash lifecycle', () => {
    async function seedScan(ocrScanStoreModule: typeof import('./ocrScanStore')) {
      const result = { text: 'X', confidence: 0.9, boxes: [] };
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_image') return result;
        return undefined;
      });
      await ocrScanStoreModule.useOcrScanStore.getState().performScan('/test.png');
    }

    it('softDeleteEntry 标记删除', async () => {
      const mod = await import('./ocrScanStore');
      await seedScan(mod);
      const id = mod.useOcrScanStore.getState().scanHistory[0].id;

      mod.useOcrScanStore.getState().softDeleteEntry(id);
      const entry = mod.useOcrScanStore.getState().scanHistory[0];
      expect(entry.isDeleted).toBe(true);
      expect(entry.deletedAt).toBeDefined();
    });

    it('getTrash 只返回已删除条目', async () => {
      const mod = await import('./ocrScanStore');
      await seedScan(mod);
      const id = mod.useOcrScanStore.getState().scanHistory[0].id;
      mod.useOcrScanStore.getState().softDeleteEntry(id);

      expect(mod.useOcrScanStore.getState().getActiveHistory()).toHaveLength(0);
      expect(mod.useOcrScanStore.getState().getTrash()).toHaveLength(1);
    });

    it('restoreEntry 恢复删除', async () => {
      const mod = await import('./ocrScanStore');
      await seedScan(mod);
      const id = mod.useOcrScanStore.getState().scanHistory[0].id;
      mod.useOcrScanStore.getState().softDeleteEntry(id);
      mod.useOcrScanStore.getState().restoreEntry(id);

      const entry = mod.useOcrScanStore.getState().scanHistory[0];
      expect(entry.isDeleted).toBe(false);
      expect(entry.deletedAt).toBeUndefined();
    });

    it('permanentlyDeleteEntry 永久删除', async () => {
      const mod = await import('./ocrScanStore');
      await seedScan(mod);
      const id = mod.useOcrScanStore.getState().scanHistory[0].id;
      mod.useOcrScanStore.getState().permanentlyDeleteEntry(id);
      expect(mod.useOcrScanStore.getState().scanHistory).toHaveLength(0);
    });

    it('clearTrash 清空所有已删除条目', async () => {
      const mod = await import('./ocrScanStore');
      await seedScan(mod);
      await seedScan(mod);
      const history = mod.useOcrScanStore.getState().scanHistory;
      history.forEach((h) => mod.useOcrScanStore.getState().softDeleteEntry(h.id));
      mod.useOcrScanStore.getState().clearTrash();
      expect(mod.useOcrScanStore.getState().scanHistory).toHaveLength(0);
    });
  });

  describe('getCurrentEntry', () => {
    it('返回当前扫描条目', async () => {
      const result = { text: 'X', confidence: 0.9, boxes: [] };
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_image') return result;
        return undefined;
      });

      const { useOcrScanStore } = await import('./ocrScanStore');
      await useOcrScanStore.getState().performScan('/img.png');
      const entry = useOcrScanStore.getState().getCurrentEntry();
      expect(entry).not.toBeNull();
      expect(entry!.fileName).toBe('img.png');
    });

    it('无扫描时返回 null', async () => {
      const { useOcrScanStore } = await import('./ocrScanStore');
      expect(useOcrScanStore.getState().getCurrentEntry()).toBeNull();
    });
  });

  describe('persistence', () => {
    it('持久化 scanHistory / activeTier / scanMode', async () => {
      const { useOcrScanStore } = await import('./ocrScanStore');
      useOcrScanStore.getState().setActiveTier('tiny');
      useOcrScanStore.getState().setScanMode('mrz');
      await new Promise((r) => setTimeout(r, 0)); // 等待 persist 写入

      const raw = localStorage.getItem('solosoul-ocr-scan-history');
      expect(raw).toBeDefined();
      const parsed = JSON.parse(raw!);
      expect(parsed.state.activeTier).toBe('tiny');
      expect(parsed.state.scanMode).toBe('mrz');
    });
  });

  // P230: Vault 锁定/退出后必须清空含解密明文的内存态（MRZ 证件号等）。
  describe('clearOnVaultLock', () => {
    it('清空扫描历史/当前条目/错误态，且保留持久化元数据', async () => {
      const mrzResult = {
        documentType: 'P',
        documentNumber: 'AB123',
        rawLines: [],
        confidence: 0.95,
        checksumValid: true,
      };
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'ocr_scan_mrz') return mrzResult;
        return undefined;
      });

      const mod = await import('./ocrScanStore');
      mod.useOcrScanStore.getState().setScanMode('mrz');
      mod.useOcrScanStore.getState().setActiveTier('tiny');
      await mod.useOcrScanStore.getState().performScan('/passport.png');

      // 锁定前：历史含 MRZ 明文
      const before = mod.useOcrScanStore.getState();
      expect(before.scanHistory).toHaveLength(1);
      expect(before.scanHistory[0].mrzResult?.documentNumber).toBe('AB123');

      mod.useOcrScanStore.getState().clearOnVaultLock();

      const after = mod.useOcrScanStore.getState();
      expect(after.scanHistory).toEqual([]);
      expect(after.currentScanId).toBeNull();
      expect(after.lastScanError).toBeNull();
      expect(after.isScanning).toBe(false);
      expect(after.isCardOpen).toBe(false);
      // 只读 UI 偏好不被清空（clearOnVaultLock 只清敏感内存态）
      expect(after.scanMode).toBe('mrz');
      expect(after.activeTier).toBe('tiny');
    });
  });
});
