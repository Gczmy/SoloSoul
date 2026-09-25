import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, act, cleanup } from '@testing-library/react';
import { setRequestSession } from '@/lib/sessionRequests';
import type { PasswordVerificationDialogProps } from '@/components/forms/PasswordVerificationDialog';

vi.mock('@/components/forms/PasswordVerificationDialog', () => ({
  PasswordVerificationDialog: ({ onVerify, onClose }: PasswordVerificationDialogProps) => (
    <div role="dialog">
      <button
        onClick={async () => {
          if (await onVerify('test-password')) onClose();
        }}
      >
        Verify
      </button>
      <button onClick={onClose}>Cancel</button>
    </div>
  ),
}));

// 共享模块引用 PAGE_ICON_MAP / searchCache，此处按真实实现使用（二者无副作用）即可。

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: vi.fn(),
}));
import { invokeCommand } from '@/lib/ipcClient';
import { searchCache } from '@/lib/searchCache';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import {
  SYSTEM_PAGE_KEYS,
  SearchItem,
  buildSearchCacheParams,
  buildSearchPayload,
  ensurePageResultExists,
  matchPageTranslation,
  resolveResultIcon,
  resolveResultName,
  runUnifiedSearch,
  sortSensitivityLevels,
  MatchHint,
  searchMatchSensitivity,
} from './searchShared';

const tMock = ((key: string, fallback?: string) => {
  // navigation:identity → "Identity" 等；其余返回 fallback 或 key
  const navMap: Record<string, string> = {
    'navigation:identity': 'Identity',
    'navigation:travel': 'Travel',
  };
  if (key in navMap) return navMap[key];
  return fallback ?? key;
}) as never;

const customPages = [
  { id: 'cp1', name: 'My Vault', iconId: 'star', deletedAt: null },
  { id: 'cp2', name: 'Deleted', iconId: 'x', deletedAt: '2026-01-01T00:00:00Z' },
] as never;

describe('searchShared helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('SYSTEM_PAGE_KEYS 覆盖五个系统页面', () => {
    expect(SYSTEM_PAGE_KEYS).toEqual([
      'identity',
      'travel',
      'financial',
      'professional',
      'document',
    ]);
  });

  it('matchPageTranslation：命中翻译后的系统页面名返回英文 key', () => {
    expect(matchPageTranslation('identity', tMock)).toBe('identity');
    expect(matchPageTranslation('Identity', tMock)).toBe('identity');
    expect(matchPageTranslation('   TRAVEL  ', tMock)).toBe('travel');
    expect(matchPageTranslation('xyz', tMock)).toBeNull();
  });

  it('resolveResultName：系统页面翻译、自定义页面用名称、对象用原始 name', () => {
    expect(
      resolveResultName(
        { itemType: 'page', objectId: 'identity', name: 'identity' },
        customPages,
        tMock,
      ),
    ).toBe('Identity');
    expect(
      resolveResultName({ itemType: 'page', objectId: 'cp1', name: 'cp1' }, customPages, tMock),
    ).toBe('My Vault');
    expect(
      resolveResultName({ itemType: 'object', objectId: 'o1', name: 'Doc' }, customPages, tMock),
    ).toBe('Doc');
  });

  it('resolveResultIcon：系统/自定义/对象图标均能解析为非空组件', () => {
    const sysIcon = resolveResultIcon(
      { itemType: 'page', typeId: 'identity', objectId: 'identity' },
      customPages,
    );
    const customIcon = resolveResultIcon(
      { itemType: 'page', typeId: 'cp1', objectId: 'cp1' },
      customPages,
    );
    const objIcon = resolveResultIcon(
      { itemType: 'object', typeId: 'unknown-ct', objectId: 'o1' },
      customPages,
    );
    expect(sysIcon).toBeTruthy();
    expect(customIcon).toBeTruthy();
    expect(objIcon).toBeTruthy();
  });

  it('resolveResultIcon：对象优先使用所属模板图标，缺失时回退所属页面图标', () => {
    // 带模板图标：即使是 identity 页面下的对象，也用模板图标而非页面图标
    const withTemplateIcon = resolveResultIcon(
      { itemType: 'object', typeId: 'identity', objectId: 'o1', templateIconId: 'star' },
      customPages,
    );
    expect(withTemplateIcon).not.toBe(PAGE_ICON_MAP.identity);
    // 无模板图标：回退到所属页面图标（既有行为）
    const fallbackIcon = resolveResultIcon(
      { itemType: 'object', typeId: 'identity', objectId: 'o2' },
      customPages,
    );
    expect(fallbackIcon).toBe(PAGE_ICON_MAP.identity);
  });

  it('sortSensitivityLevels：按 public→critical 升序', () => {
    expect(sortSensitivityLevels(['critical', 'public', 'sensitive', 'internal'])).toEqual([
      'public',
      'internal',
      'sensitive',
      'critical',
    ]);
    // 未知级别被过滤
    expect(sortSensitivityLevels(['bogus', 'public'])).toEqual(['public']);
  });

  it('buildSearchPayload：pageKey 优先、自定义页走 parentId、系统页走 typeId', () => {
    expect(buildSearchPayload('a', 'q', 'identity', null, customPages)).toEqual({
      accountId: 'a',
      query: 'q',
      limit: 50,
      typeId: 'identity',
    });
    expect(buildSearchPayload('a', 'q', null, 'cp1', customPages)).toEqual({
      accountId: 'a',
      query: 'q',
      limit: 50,
      parentId: 'cp1',
    });
    expect(buildSearchPayload('a', 'q', null, 'travel', customPages)).toEqual({
      accountId: 'a',
      query: 'q',
      limit: 50,
      typeId: 'travel',
    });
    // 无 pageKey 无 filter
    expect(buildSearchPayload('a', 'q', null, null, customPages)).toEqual({
      accountId: 'a',
      query: 'q',
      limit: 50,
    });
  });

  it('buildSearchCacheParams：与 payload 同一参数派生缓存键', () => {
    const params = buildSearchCacheParams('a', 'q', null, 'cp1', customPages);
    expect(params.parentId).toBe('cp1');
    expect(params.effectiveCollectionType).toBeNull();
    expect(typeof params.cacheKey).toBe('string');

    const params2 = buildSearchCacheParams('a', 'q', 'identity', null, customPages);
    expect(params2.effectiveCollectionType).toBe('identity');
  });

  it('ensurePageResultExists：缺页面时置顶合成 page 结果', () => {
    const items = [{ objectId: 'o1', name: 'O1', typeId: 'x', relevance: 1 }] as SearchItem[];
    const result = ensurePageResultExists(items, 'identity');
    expect(result.length).toBe(2);
    expect(result[0].itemType).toBe('page');
    expect(result[0].objectId).toBe('identity');
    expect(result[0].relevance).toBe(99);

    // 已存在则不重复
    const withPage = [
      {
        objectId: 'identity',
        name: 'identity',
        typeId: 'identity',
        itemType: 'page',
        relevance: 1,
      },
    ] as SearchItem[];
    const result2 = ensurePageResultExists(withPage, 'identity');
    expect(result2.length).toBe(1);
  });

  it('MatchHint：字段值命中时渲染高亮', () => {
    render(
      <MatchHint
        item={
          {
            matchedField: 'contact.email',
            matchedValue: 'alice@example.com',
            matchType: 'fieldValue',
            itemType: 'object',
            sensitivityLevels: ['public'],
          } as SearchItem
        }
        query="alice"
        t={tMock}
      />,
    );
    const mark = screen.getByText('alice');
    expect(mark.tagName).toBe('MARK');
    // 整体文本被 Highlight 拆分渲染，用 textContent 汇总断言
    expect(mark.closest('span')?.textContent).toContain('alice@example.com');
  });

  it('MatchHint：page / 无命中时返回空', () => {
    const { container } = render(
      <MatchHint
        item={{ itemType: 'page', matchedField: 'x' } as SearchItem}
        query="q"
        t={tMock}
      />,
    );
    expect(container.innerHTML).toBe('');
  });
});

describe('runUnifiedSearch（只返回结果）', () => {
  const params = { accountId: 'acc-1', query: 'xyz', filter: null, customPages, t: tMock };
  beforeEach(() => {
    searchCache.clear();
    vi.mocked(invokeCommand).mockReset();
    vi.mocked(invokeCommand).mockResolvedValue({ items: [], total: 0, hasMore: false });
  });
  it('空查询且无 filter 不发请求', async () => {
    expect(await runUnifiedSearch({ ...params, query: ' ' })).toMatchObject({
      items: [],
      hasSearched: false,
    });
    expect(invokeCommand).not.toHaveBeenCalled();
  });
  it('有 filter 的空查询仍发起搜索', async () => {
    await runUnifiedSearch({ ...params, query: '', filter: 'cp1' });
    expect(invokeCommand).toHaveBeenCalledWith(
      'search_unified',
      expect.objectContaining({ parentId: 'cp1' }),
    );
  });
  it('成功只返回结果，不在会话校验前写缓存', async () => {
    const items = [{ objectId: 'o1', name: 'Doc', typeId: 'note', relevance: 5 }];
    vi.mocked(invokeCommand).mockResolvedValue({ items });
    const result = await runUnifiedSearch(params);
    expect(result.items).toEqual(items);
    expect(result.hasSearched).toBe(true);
    expect(searchCache.get(result.cacheKey!)).toBeNull();
  });
  it('失败交给调用方判定是否仍应显示错误', async () => {
    vi.mocked(invokeCommand).mockRejectedValue(new Error('boom'));
    await expect(runUnifiedSearch(params)).rejects.toThrow('boom');
  });
  it('系统页名查询仍合成缺失的页面项', async () => {
    const result = await runUnifiedSearch({ ...params, query: 'Identity' });
    expect(result.items).toEqual([
      expect.objectContaining({ objectId: 'identity', itemType: 'page' }),
    ]);
  });
  it('缓存命中不重复请求', async () => {
    const items = [{ objectId: 'cached', name: 'Doc', typeId: 'note', relevance: 5 }];
    searchCache.set(searchCache.buildKey('acc-1', 'xyz'), items);
    expect(await runUnifiedSearch(params)).toMatchObject({ items, cached: true });
    expect(invokeCommand).not.toHaveBeenCalled();
  });
});

describe('RF-108 protected match values', () => {
  const item: SearchItem = {
    objectId: 'one',
    name: 'Object',
    typeId: 'identity',
    itemType: 'object',
    relevance: 1,
    matchType: 'fieldValue',
    matchedField: 'credential',
    matchedValue: 'TOP_SECRET',
    sensitivityLevels: ['critical'],
  };
  beforeEach(() => {
    vi.mocked(invokeCommand).mockReset();
    setRequestSession('a');
  });
  afterEach(() => {
    cleanup();
    setRequestSession(null);
    vi.useRealTimers();
  });
  it.each([
    { levels: ['public'], expected: 'public' },
    { levels: ['internal'], expected: 'internal' },
    { levels: ['sensitive'], expected: 'sensitive' },
    { levels: ['critical'], expected: 'critical' },
    { levels: ['public', 'critical', 'internal'], expected: 'critical' },
    { levels: ['public', 'unknown'], expected: 'internal' },
    { levels: [], expected: 'internal' },
  ])(
    'uses aggregate $expected for $levels without guessing the matching field',
    async ({ levels, expected }) => {
      expect(searchMatchSensitivity(levels)).toBe(expected);
      const navigate = vi.fn();
      const { container } = render(
        <div onClick={navigate} onKeyDown={navigate}>
          <MatchHint
            item={{ ...item, sensitivityLevels: levels }}
            query="TOP"
            t={tMock}
            accountId="a"
          />
        </div>,
      );
      expect(container.textContent?.includes('TOP_SECRET')).toBe(expected === 'public');
      if (expected !== 'public') {
        const reveal = screen.getByText('••••••••');
        expect(reveal.tagName).toBe('BUTTON');
        expect(reveal).toHaveAccessibleName();
        fireEvent.keyDown(reveal, { key: 'Enter' });
        await act(async () => fireEvent.click(reveal));
        expect(navigate).not.toHaveBeenCalled();
        if (expected === 'critical') {
          expect(screen.getByRole('dialog')).toBeInTheDocument();
          expect(container.innerHTML).not.toContain('TOP_SECRET');
        } else expect(container.textContent).toContain('TOP_SECRET');
      }
    },
  );

  it('cancel and incorrect password stay concealed; success audits and expires after TTL', async () => {
    vi.mocked(invokeCommand).mockImplementation(async (cmd) =>
      cmd === 'verify_password' ? false : undefined,
    );
    const { container } = render(<MatchHint item={item} query="" t={tMock} accountId="a" />);
    await act(async () => fireEvent.click(screen.getByText('••••••••')));
    await act(async () => fireEvent.click(screen.getByText('Cancel')));
    expect(container.innerHTML).not.toContain('TOP_SECRET');
    await act(async () => fireEvent.click(screen.getByText('••••••••')));
    await act(async () => fireEvent.click(screen.getByText('Verify')));
    expect(container.innerHTML).not.toContain('TOP_SECRET');
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    vi.mocked(invokeCommand).mockImplementation(async (cmd) =>
      cmd === 'verify_password' ? true : undefined,
    );
    vi.useFakeTimers();
    await act(async () => fireEvent.click(screen.getByText('Verify')));
    expect(container.textContent).toContain('TOP_SECRET');
    expect(vi.mocked(invokeCommand).mock.calls.filter(([cmd]) => cmd === 'log_write')).toHaveLength(
      1,
    );
    act(() => vi.advanceTimersByTime(60_001));
    expect(container.innerHTML).not.toContain('TOP_SECRET');
  });

  it.each(['query', 'value', 'account', 'lock', 'unmount'] as const)(
    '%s changes cancel pending verification',
    async (change) => {
      let finish!: (ok: boolean) => void;
      vi.mocked(invokeCommand).mockImplementation(async (cmd) =>
        cmd === 'verify_password'
          ? new Promise<boolean>((resolve) => {
              finish = resolve;
            })
          : undefined,
      );
      const { rerender, unmount } = render(
        <MatchHint item={item} query="one" t={tMock} accountId="a" />,
      );
      await act(async () => fireEvent.click(screen.getByText('••••••••')));
      fireEvent.click(screen.getByText('Verify'));
      if (change === 'unmount') unmount();
      else if (change === 'lock')
        act(() => {
          setRequestSession(null);
          setRequestSession('a');
        });
      else
        rerender(
          <MatchHint
            item={{ ...item, matchedValue: change === 'value' ? 'OTHER_SECRET' : 'TOP_SECRET' }}
            query={change === 'query' ? 'two' : 'one'}
            t={tMock}
            accountId={change === 'account' ? 'b' : 'a'}
          />,
        );
      await act(async () => finish(true));
      expect(document.body.innerHTML).not.toContain('TOP_SECRET');
      expect(document.body.innerHTML).not.toContain('OTHER_SECRET');
      expect(
        vi.mocked(invokeCommand).mock.calls.filter(([cmd]) => cmd === 'log_write'),
      ).toHaveLength(0);
    },
  );
});
