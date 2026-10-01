import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ObjectSummary } from '@/stores/objectStore';
import type { ChatMsg } from '@/types/llmChat';
import { buildChatRequest } from './chatRequest';
import { searchGuideChunks, type GuideChunk } from './guideService';

const state = vi.hoisted(() => ({
  objects: [] as ObjectSummary[],
  language: 'zh-CN',
  readObjects: vi.fn(),
  readSensitiveStore: vi.fn(),
}));
vi.mock('@/lib/i18n', () => ({
  default: {
    get language() {
      return state.language;
    },
  },
}));
vi.mock('@/stores/objectStore', () => ({ useObjectStore: { getState: state.readObjects } }));
vi.mock('@/stores/templateStore', () => ({
  useTemplateStore: { getState: state.readSensitiveStore },
}));
vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: { getState: state.readSensitiveStore },
}));
vi.mock('./guideService', () => ({ searchGuideChunks: vi.fn() }));

function object(id: string, sensitivityLevel = 'public', isDeleted = false): ObjectSummary {
  const summary = { id, sensitivityLevel, isDeleted } as ObjectSummary;
  // 选择阶段只能读取 ID 与可见性；即使公开对象也不得读取正文或描述字段。
  for (const field of ['name', 'properties', 'propertyLabels', 'templateId', 'typeId']) {
    Object.defineProperty(summary, field, {
      enumerable: true,
      get() {
        throw new Error('Unexpected object field read: ' + field);
      },
    });
  }
  return summary;
}

beforeEach(() => {
  vi.clearAllMocks();
  state.objects = [];
  state.language = 'zh-CN';
  state.readObjects.mockImplementation(() => ({ objects: state.objects }));
  state.readSensitiveStore.mockImplementation(() => {
    throw new Error('Unexpected template/settings read');
  });
  vi.mocked(searchGuideChunks).mockResolvedValue([]);
});

describe('RF-004 context selection contains references only', () => {
  it('selects only the first three loaded public non-deleted IDs without reading fields', async () => {
    state.objects = [
      object('internal', 'internal'),
      object('deleted', 'public', true),
      object('public-1'),
      object('sensitive', 'sensitive'),
      object('public-2'),
      object('critical', 'critical'),
      object('public-3'),
      object('public-4'),
    ];
    const request = await buildChatRequest({
      accountId: 'account',
      text: '你好',
      history: [],
      includeSystemPrompt: true,
    });
    expect(request).toEqual({
      messages: [{ role: 'user', content: '你好' }],
      contextSelection: {
        mode: 'publicProfile',
        objectIds: ['public-1', 'public-2', 'public-3'],
        language: 'zh-CN',
        guideChunks: [],
      },
    });
    expect(state.readObjects).toHaveBeenCalledOnce();
    expect(state.readSensitiveStore).not.toHaveBeenCalled();
  });

  it('does not read object data or retrieve guides when context is disabled', async () => {
    state.readObjects.mockImplementation(() => {
      throw new Error('Disabled context must not read ObjectStore');
    });
    const request = await buildChatRequest({
      accountId: 'account',
      text: '你好',
      history: [],
      includeSystemPrompt: false,
    });
    expect(request).toEqual({
      messages: [{ role: 'user', content: '你好' }],
      contextSelection: { mode: 'none' },
    });
    expect(state.readObjects).not.toHaveBeenCalled();
    expect(state.readSensitiveStore).not.toHaveBeenCalled();
    expect(searchGuideChunks).not.toHaveBeenCalled();
  });

  it('preserves an empty object selection without widening it to other objects', async () => {
    state.objects = [object('private', 'sensitive'), object('deleted', 'public', true)];
    const request = await buildChatRequest({
      accountId: 'account',
      text: '你好',
      history: [],
      includeSystemPrompt: true,
    });
    expect(request.contextSelection).toEqual({
      mode: 'publicProfile',
      objectIds: [],
      language: 'zh-CN',
      guideChunks: [],
    });
  });

  it('captures language and IDs before guide retrieval and forwards unwrapped chunks', async () => {
    state.objects = [object('selected')];
    let resolveGuides!: (chunks: GuideChunk[]) => void;
    const pendingGuides = new Promise<GuideChunk[]>((resolve) => {
      resolveGuides = resolve;
    });
    vi.mocked(searchGuideChunks).mockReturnValue(pendingGuides);
    const pendingRequest = buildChatRequest({
      accountId: 'account',
      text: '如何使用',
      history: [],
      includeSystemPrompt: true,
    });
    state.language = 'en-US';
    state.objects = [object('later')];
    const guideChunks = [
      { guideId: 'guide', guideTitle: '使用指南', chunkText: '官方帮助正文', similarity: 0.75 },
    ];
    resolveGuides(guideChunks);
    expect(searchGuideChunks).toHaveBeenCalledWith(
      'account',
      '如何使用',
      'zh-CN',
      expect.objectContaining({
        assertCurrent: expect.any(Function),
        invokeTyped: expect.any(Function),
      }),
    );
    expect((await pendingRequest).contextSelection).toEqual({
      mode: 'publicProfile',
      objectIds: ['selected'],
      language: 'zh-CN',
      guideChunks,
    });
  });

  it.each([false, true])(
    'filters legacy roles only in the outbound copy (context=%s)',
    async (includeSystemPrompt) => {
      const history: ChatMsg[] = [
        { id: 'system', role: 'system', content: 'LEGACY_SYSTEM', createdAt: '' },
        { id: 'user', role: 'user', content: '旧问题', createdAt: '' },
        { id: 'tool', role: 'tool', content: 'LEGACY_TOOL', createdAt: '' },
        { id: 'assistant', role: 'assistant', content: '旧回答', createdAt: '' },
        { id: 'unknown', role: 'unknown', content: 'LEGACY_UNKNOWN', createdAt: '' },
      ];
      const snapshot = structuredClone(history);
      const request = await buildChatRequest({
        accountId: 'account',
        text: '新问题',
        history,
        includeSystemPrompt,
      });
      expect(request.messages).toEqual([
        { role: 'user', content: '旧问题' },
        { role: 'assistant', content: '旧回答' },
        { role: 'user', content: '新问题' },
      ]);
      expect(history).toEqual(snapshot);
      expect(request.messages[0]).not.toBe(history[1]);
    },
  );
});
