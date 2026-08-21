import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { FieldSuggestions, type FieldSuggestion } from './FieldSuggestions';
import { MASK_PLACEHOLDER } from '@/lib/masking';

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
  it('无推荐时不渲染任何内容', () => {
    const { container } = render(
      <FieldSuggestions fieldName="身份证号码" suggestions={[]} onPick={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
    expect(screen.queryByTestId('field-suggestions')).not.toBeInTheDocument();
  });

  it('展示 [对象名][敏感度徽章][遮掩内容]，非 public 一律 8 圆点掩码', () => {
    const onPick = vi.fn();
    render(
      <FieldSuggestions
        fieldName="身份证号码"
        suggestions={[
          makeSuggestion({ objectName: '我的身份证', sensitivityLevel: 'critical' }),
          makeSuggestion({
            objectId: 'obj-2',
            objectName: '身份信息',
            sensitivityLevel: 'internal',
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
    // 遮掩内容：critical 与 internal 均显示统一占位符，而非真实值
    expect(screen.getAllByText(MASK_PLACEHOLDER)).toHaveLength(2);
    expect(screen.queryByText('110101199001011234')).not.toBeInTheDocument();
    // 敏感度徽章（图标模式）：title 含等级
    expect(screen.getByTitle(/critical/)).toBeInTheDocument();
    expect(screen.getByTitle(/internal/)).toBeInTheDocument();
  });

  it('public 字段明文展示（截断超长值）', () => {
    render(
      <FieldSuggestions
        fieldName="备注"
        suggestions={[makeSuggestion({ sensitivityLevel: 'public', value: 'x'.repeat(200) })]}
        onPick={vi.fn()}
      />,
    );
    // 截断到 80 字符 + 省略号
    expect(screen.getByText(`${'x'.repeat(80)}…`)).toBeInTheDocument();
    expect(screen.queryByText('x'.repeat(200))).not.toBeInTheDocument();
  });

  it('点击条目回填真实值（即使展示为掩码）', () => {
    const onPick = vi.fn();
    render(
      <FieldSuggestions
        fieldName="身份证号码"
        suggestions={[makeSuggestion({ value: '110101199001011234' })]}
        onPick={onPick}
      />,
    );
    fireEvent.click(screen.getByTestId('field-suggestion-item'));
    expect(onPick).toHaveBeenCalledWith('110101199001011234');
  });

  it('超出 limit 时折叠为「还有 N 条」', () => {
    const many = Array.from({ length: 7 }, (_, i) =>
      makeSuggestion({ objectId: `obj-${i}`, objectName: `对象${i}` }),
    );
    render(
      <FieldSuggestions fieldName="身份证号码" suggestions={many} onPick={vi.fn()} limit={5} />,
    );
    expect(screen.getAllByTestId('field-suggestion-item')).toHaveLength(5);
    expect(screen.getByText('+2 more')).toBeInTheDocument();
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
});
