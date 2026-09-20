import { useState } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SelectCheckbox } from './SelectCheckbox';

describe('SelectCheckbox 原生交互', () => {
  it('点击控件和关联 label 文本都只切换一次', () => {
    const onChange = vi.fn();
    function Form() {
      const [checked, setChecked] = useState(false);
      return (
        <label>
          <SelectCheckbox
            checked={checked}
            onChange={(value) => {
              onChange(value);
              setChecked(value);
            }}
          />
          <span>导出附件</span>
        </label>
      );
    }
    render(<Form />);
    const checkbox = screen.getByRole('checkbox', { name: '导出附件' });
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
    expect(onChange.mock.calls).toEqual([[true]]);
    fireEvent.click(screen.getByText('导出附件'));
    expect(checkbox).not.toBeChecked();
    expect(onChange.mock.calls).toEqual([[true], [false]]);
  });

  it('有选择回调的控件不重复触发父行点击', () => {
    const onChange = vi.fn();
    const onRowClick = vi.fn();
    render(
      <div onClick={onRowClick}>
        <SelectCheckbox checked={false} onChange={onChange} aria-label="选择对象" />
        <span>打开对象</span>
      </div>,
    );
    fireEvent.click(screen.getByRole('checkbox', { name: '选择对象' }));
    expect(onChange).toHaveBeenCalledExactlyOnceWith(true);
    expect(onRowClick).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText('打开对象'));
    expect(onRowClick).toHaveBeenCalledTimes(1);
  });

  it('Space/Enter 不冒泡到父卡片，同时保留浏览器默认键盘行为', () => {
    const onRowKeyDown = vi.fn();
    render(
      <div onKeyDown={onRowKeyDown}>
        <SelectCheckbox checked={false} onChange={vi.fn()} aria-label="选择对象" />
      </div>,
    );
    const checkbox = screen.getByRole('checkbox');
    checkbox.focus();
    expect(checkbox).toHaveFocus();
    for (const key of [' ', 'Enter']) {
      const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
      fireEvent(checkbox, event);
      expect(event.defaultPrevented).toBe(false);
    }
    expect(onRowKeyDown).not.toHaveBeenCalled();
  });

  it('混合状态同步到原生 indeterminate，点击选择全部并可清除混合状态', () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <SelectCheckbox checked={false} indeterminate onChange={onChange} aria-label="全选" />,
    );
    const checkbox = screen.getByRole('checkbox', { name: '全选' });
    expect(checkbox).toBePartiallyChecked();
    expect(checkbox).toHaveProperty('indeterminate', true);
    fireEvent.click(checkbox);
    expect(onChange).toHaveBeenCalledExactlyOnceWith(true);
    rerender(<SelectCheckbox checked onChange={onChange} aria-label="全选" />);
    expect(checkbox).toBeChecked();
    expect(checkbox).not.toBePartiallyChecked();
    expect(checkbox).toHaveProperty('indeterminate', false);
  });

  it('禁用时点击控件和目标区都不触发选择或父行', () => {
    const onChange = vi.fn();
    const onRowClick = vi.fn();
    render(
      <div onClick={onRowClick}>
        <SelectCheckbox checked={false} disabled onChange={onChange} aria-label="不可选择" />
      </div>,
    );
    const checkbox = screen.getByRole('checkbox', { name: '不可选择' });
    expect(checkbox).toBeDisabled();
    fireEvent.click(checkbox);
    fireEvent.click(checkbox.parentElement!);
    expect(onChange).not.toHaveBeenCalled();
    expect(onRowClick).not.toHaveBeenCalled();
  });

  it('兼容旧 onClick，且支持外部 htmlFor 标签', () => {
    const onClick = vi.fn();
    const onRowClick = vi.fn();
    render(
      <>
        <div onClick={onRowClick}>
          <SelectCheckbox id="legacy-select" checked={false} onClick={onClick} />
        </div>
        <label htmlFor="legacy-select">选择附件</label>
      </>,
    );
    fireEvent.click(screen.getByRole('checkbox', { name: '选择附件' }));
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(onRowClick).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText('选择附件'));
    expect(onClick).toHaveBeenCalledTimes(2);
    expect(onRowClick).not.toHaveBeenCalled();
  });
});
