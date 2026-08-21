import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { FieldSuggestions, type FieldSuggestion } from './FieldSuggestions';
import { MASK_PLACEHOLDER } from '@/lib/masking';

// ── 依赖 mock ────────────────────────────────────────────────────────────
const { handleItemClickMock, handleFillClickMock, revealedIds, revealTimes } = vi.hoisted(() => {
  const revealTimes: Record<string, number> = {};
  return {
    handleItemClickMock: vi.fn<(item: FieldSuggestion) => void>(),
    handleFillClickMock: vi.fn<(item: FieldSuggestion, onPick: (v: string) => void) => void>(),
    revealedIds: new Set<string>(),
    revealTimes,
  };
});

vi.mock('@/components/editor/useSuggestionReveal', () => ({
  suggestionItemId: (item: FieldSuggestion) => `${item.objectId}::${item.fieldKey}`,
  useSuggestionReveal: () => ({
    isRevealed: (id: string) => revealedIds.has(id),
    revealRemainingMs: (id: string) => {
      const start = revealTimes[id];
      if (!start || !revealedIds.has(id)) return 0;
      return Math.max(0, 60_000 - (Date.now() - start));
    },
    handleItemClick: handleItemClickMock,
    handleFillClick: handleFillClickMock,
    showPwDialog: false,
    handlePwDialogClose: vi.fn(),
    handlePwDialogVerify: vi.fn(),
    handlePwDialogPinSuccess: vi.fn(),
    passwordHint: null,
    bioAvailable: { available: false },
    handleBiometricUnlock: vi.fn(),
  }),
}));

vi.mock('@/stores/authStore', () => ({
  useAuthStore: (selector: (s: { currentAccount: { id: string } | null }) => unknown) =>
    selector({ currentAccount: { id: 'acc-1' } }),
}));

vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

function makeSuggestion(overrides: Partial<FieldSuggestion> = {}): FieldSuggestion {
  return {
    objectId: 'obj-1',
    objectName: '我的身份证',
    fieldKey: 'citizen_no',
    fieldName: '身份证号码',
    sensitivityLevel: 'critical',
    value: '110101199001011234',
    ...overrides,
  };
}

describe('FieldSuggestions', () => {
  beforeEach(() => {
    handleItemClickMock.mockClear();
    handleFillClickMock.mockClear();
    revealedIds.clear();
    for (const k of Object.keys(revealTimes)) delete revealTimes[k];
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('无推荐时不渲染任何内容', () => {
    const { container } = render(
      <FieldSuggestions fieldName="身份证号码" suggestions={[]} onPick={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
    expect(screen.queryByTestId('field-suggestions')).not.toBeInTheDocument();
  });

  it('展示 [对象名][敏感度徽章][遮掩内容]，sensitive/critical 一律 8 圆点掩码', () => {
    const onPick = vi.fn();
    render(
      <FieldSuggestions
        fieldName="身份证号码"
        suggestions={[
          makeSuggestion({ objectName: '我的身份证', sensitivityLevel: 'critical' }),
          makeSuggestion({
            objectId: 'obj-2',
            objectName: '身份信息',
            sensitivityLevel: 'sensitive',
            value: 'abc',
          }),
        ]}
        onPick={onPick}
      />,
    );

    const items = screen.getAllByTestId('field-suggestion-item');
    expect(items).toHaveLength(2);
    expect(screen.getByText('我的身份证')).toBeInTheDocument();
    expect(screen.getByText('身份信息')).toBeInTheDocument();
    // 遮掩内容：critical 与 sensitive 均显示统一占位符，而非真实值
    expect(screen.getAllByText(MASK_PLACEHOLDER)).toHaveLength(2);
    expect(screen.queryByText('110101199001011234')).not.toBeInTheDocument();
    // 敏感度徽章（图标模式）：title 含等级
    expect(screen.getByTitle(/critical/)).toBeInTheDocument();
    expect(screen.getByTitle(/sensitive/)).toBeInTheDocument();
  });

  it('public 与 internal 字段直接明文展示（截断超长值）', () => {
    render(
      <FieldSuggestions
        fieldName="备注"
        suggestions={[
          makeSuggestion({ sensitivityLevel: 'public', value: 'x'.repeat(200) }),
          makeSuggestion({ objectId: 'obj-2', sensitivityLevel: 'internal', value: '内务备注' }),
        ]}
        onPick={vi.fn()}
      />,
    );
    // public：截断到 80 字符 + 省略号
    expect(screen.getByText(`${'x'.repeat(80)}…`)).toBeInTheDocument();
    expect(screen.queryByText('x'.repeat(200))).not.toBeInTheDocument();
    // internal：直接明文，不掩码
    expect(screen.getByText('内务备注')).toBeInTheDocument();
    expect(screen.queryByText(MASK_PLACEHOLDER)).not.toBeInTheDocument();
  });

  it('点击非 public 条目走揭示逻辑（不直接回填），揭示态显示明文', () => {
    const onPick = vi.fn();
    const suggestion = makeSuggestion({ sensitivityLevel: 'sensitive', value: 'a@b.com' });
    const { rerender } = render(
      <FieldSuggestions fieldName="邮箱" suggestions={[suggestion]} onPick={onPick} />,
    );
    expect(screen.getByText(MASK_PLACEHOLDER)).toBeInTheDocument();
    expect(screen.queryByText('a@b.com')).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId('field-suggestion-item'));
    expect(handleItemClickMock).toHaveBeenCalledWith(suggestion);
    expect(onPick).not.toHaveBeenCalled();

    // 揭示态：真实值展示（由 hook 的 isRevealed 驱动）
    revealedIds.add('obj-1::citizen_no');
    rerender(<FieldSuggestions fieldName="邮箱" suggestions={[suggestion]} onPick={onPick} />);
    expect(screen.queryByText(MASK_PLACEHOLDER)).not.toBeInTheDocument();
    expect(screen.getByText('a@b.com')).toBeInTheDocument();
  });

  it('critical 条目行标题提示需验证主密码', () => {
    render(
      <FieldSuggestions
        fieldName="身份证号码"
        suggestions={[makeSuggestion({ sensitivityLevel: 'critical' })]}
        onPick={vi.fn()}
      />,
    );
    // 全局 react-i18next mock 下 t 返回 defaultValue（行容器与眼睛图标各带 title）
    expect(screen.getAllByTitle('Verify master password to view').length).toBeGreaterThan(0);
  });

  it('点击「填入」按钮走 handleFillClick（解锁/回填决策由 hook 负责）', () => {
    const onPick = vi.fn();
    const suggestion = makeSuggestion({ value: '110101199001011234' });
    render(
      <FieldSuggestions
        fieldName="身份证号码"
        suggestions={[suggestion]}
        onPick={onPick}
      />,
    );
    fireEvent.click(screen.getByTestId('field-suggestion-fill'));
    expect(handleFillClickMock).toHaveBeenCalledWith(suggestion, onPick);
    // 填入按钮不直接回填、不触发条目揭示
    expect(onPick).not.toHaveBeenCalled();
    expect(handleItemClickMock).not.toHaveBeenCalled();
  });

  it('超过 limit 时默认折叠为 limit 条，提供展开/收起按钮', () => {
    const many = Array.from({ length: 7 }, (_, i) =>
      makeSuggestion({ objectId: `obj-${i}`, objectName: `对象${i}` }),
    );
    render(
      <FieldSuggestions fieldName="身份证号码" suggestions={many} onPick={vi.fn()} />,
    );
    // 默认 limit=3：折叠态只显示 3 条
    expect(screen.getAllByTestId('field-suggestion-item')).toHaveLength(3);
    const toggle = screen.getByTestId('field-suggestions-toggle');
    expect(toggle).toHaveTextContent('Expand (4 more)');

    // 展开：显示全部 7 条，按钮变为收起
    fireEvent.click(toggle);
    expect(screen.getAllByTestId('field-suggestion-item')).toHaveLength(7);
    expect(screen.getByTestId('field-suggestions-toggle')).toHaveTextContent('Collapse');

    // 收起：回到 3 条
    fireEvent.click(screen.getByTestId('field-suggestions-toggle'));
    expect(screen.getAllByTestId('field-suggestion-item')).toHaveLength(3);
  });

  it('不超过 limit 时不显示展开按钮', () => {
    const few = Array.from({ length: 3 }, (_, i) =>
      makeSuggestion({ objectId: `obj-${i}`, objectName: `对象${i}` }),
    );
    render(<FieldSuggestions fieldName="身份证号码" suggestions={few} onPick={vi.fn()} />);
    expect(screen.getAllByTestId('field-suggestion-item')).toHaveLength(3);
    expect(screen.queryByTestId('field-suggestions-toggle')).not.toBeInTheDocument();
  });

  it('自定义 limit 与掩码：sensitive 级别同样遮掩', () => {
    render(
      <FieldSuggestions
        fieldName="邮箱"
        suggestions={[makeSuggestion({ sensitivityLevel: 'sensitive', value: 'a@b.com' })]}
        onPick={vi.fn()}
        limit={1}
      />,
    );
    expect(screen.getByText(MASK_PLACEHOLDER)).toBeInTheDocument();
    expect(screen.queryByText('a@b.com')).not.toBeInTheDocument();
  });

  it('internal 条目无揭示交互（行按钮禁用、无眼睛图标）', () => {
    render(
      <FieldSuggestions
        fieldName="备注"
        suggestions={[makeSuggestion({ sensitivityLevel: 'internal', value: '内务备注' })]}
        onPick={vi.fn()}
      />,
    );
    const row = screen.getByTestId('field-suggestion-item');
    expect(row).toBeDisabled();
    expect(screen.queryByTitle('Click to view plaintext')).not.toBeInTheDocument();
  });

  it('揭示中的条目显示自动隐藏倒计时（每秒递减），掩码时不显示', () => {
    vi.useFakeTimers();
    const suggestion = makeSuggestion({ sensitivityLevel: 'critical' });
    const { rerender } = render(
      <FieldSuggestions fieldName="身份证号码" suggestions={[suggestion]} onPick={vi.fn()} />,
    );
    // 掩码态：无倒计时
    expect(screen.queryByTestId('field-suggestion-countdown')).not.toBeInTheDocument();

    // 揭示态：显示剩余秒数
    const id = 'obj-1::citizen_no';
    revealedIds.add(id);
    revealTimes[id] = Date.now();
    rerender(<FieldSuggestions fieldName="身份证号码" suggestions={[suggestion]} onPick={vi.fn()} />);
    expect(screen.getByTestId('field-suggestion-countdown')).toHaveTextContent('60s');

    // 1 秒后跳动为 59s
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.getByTestId('field-suggestion-countdown')).toHaveTextContent('59s');
  });

  it('public 条目即使揭示也不显示倒计时', () => {
    const suggestion = makeSuggestion({ sensitivityLevel: 'public', value: '公开值' });
    const { rerender } = render(
      <FieldSuggestions fieldName="备注" suggestions={[suggestion]} onPick={vi.fn()} />,
    );
    expect(screen.queryByTestId('field-suggestion-countdown')).not.toBeInTheDocument();

    const id = 'obj-1::citizen_no';
    revealedIds.add(id);
    revealTimes[id] = Date.now();
    rerender(<FieldSuggestions fieldName="备注" suggestions={[suggestion]} onPick={vi.fn()} />);
    // public 不掩码、无揭示态，不显示倒计时
    expect(screen.queryByTestId('field-suggestion-countdown')).not.toBeInTheDocument();
  });
});
