import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// 模拟 ipc commands
const mockScanImage = vi.fn();
const mockScanMrz = vi.fn();
vi.mock('@/lib/ipc', () => ({
  commands: {
    ocrScanImage: (...args: unknown[]) => mockScanImage(...args),
    ocrScanMrz: (...args: unknown[]) => mockScanMrz(...args),
  },
  OcrResult: {} as never, // type only
  MrzResult: {} as never,
}));

// 模拟 localStorage（用于 zustand persist）
function createLocalStorageMock() {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
}

describe('ocrScanStore', () => {
  let storage: ReturnType<typeof createLocalStorageMock>;

  beforeEach(() => {
    storage = createLocalStorageMock();
    vi.stubGlobal('localStorage', storage);
    mockScanImage.mockReset();
    mockScanMrz.mockReset();
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
      mockScanImage.mockResolvedValue(result);

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
      const mrzResult = { documentType: 'P', documentNumber: 'AB123', rawLines: [], confidence: 0.95, checksumValid: true };
      mockScanMrz.mockResolvedValue(mrzResult);

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
      mockScanMrz.mockResolvedValue(null); // MRZ 无结果
      const fallback = { text: 'Fallback', confidence: 0.8, boxes: [] };
      mockScanImage.mockResolvedValue(fallback);

      const { useOcrScanStore } = await import('./ocrScanStore');
      useOcrScanStore.getState().setScanMode('mrz');
      await useOcrScanStore.getState().performScan('/no-mrz.png');

      const state = useOcrScanStore.getState();
      expect(mockScanImage).toHaveBeenCalled(); // fallback 被调用
      expect(state.scanHistory[0].result).toEqual(fallback); // 存储在 result 而非 mrzResult
    });

    it('扫描失败时设置错误信息', async () => {
      mockScanImage.mockRejectedValue(new Error('OCR engine error'));

      const { useOcrScanStore } = await import('./ocrScanStore');
      await useOcrScanStore.getState().performScan('/bad.png');

      const state = useOcrScanStore.getState();
      expect(state.isScanning).toBe(false);
      expect(state.lastScanError).toBe('OCR engine error');
      expect(state.scanHistory[0].error).toBe('OCR engine error');
    });

    it('历史记录不超过 50 条', async () => {
      const result = { text: 'X', confidence: 0.9, boxes: [] };
      mockScanImage.mockResolvedValue(result);

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
      mockScanImage.mockResolvedValue(result);
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
      mockScanImage.mockResolvedValue(result);

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
});
