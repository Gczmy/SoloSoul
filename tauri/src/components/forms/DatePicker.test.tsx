import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DatePicker } from './DatePicker';

describe('DatePicker', () => {
  it('renders segment placeholders when no value is provided', () => {
    render(<DatePicker onChange={vi.fn()} />);
    expect(screen.getByPlaceholderText('yyyy')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('mm')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('dd')).toBeInTheDocument();
  });

  it('opens calendar when a segment input is focused', () => {
    render(<DatePicker onChange={vi.fn()} />);
    fireEvent.focus(screen.getByLabelText('年份输入'));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('opens calendar when clicking anywhere in the input area', () => {
    render(<DatePicker onChange={vi.fn()} />);
    // fireEvent.click 不触发 focus，验证走 triggerRow 行级 onClick 的路径
    fireEvent.click(screen.getByLabelText('年份输入'));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('clear button does not open the calendar', () => {
    render(<DatePicker value="2020-02-15" onChange={vi.fn()} />);
    fireEvent.click(screen.getByLabelText('清除'));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('selects a date and calls onChange with YYYY-MM-DD', async () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);
    fireEvent.click(screen.getByLabelText('年份输入'));

    // Open year dropdown and pick 2020（限定到下拉选项，防止与触发器当前年份撞车）
    const yearSelect = screen.getByLabelText('选择年份');
    fireEvent.click(yearSelect);
    fireEvent.click(screen.getByText('2020', { selector: '[data-dd-value="2020"]' }));

    // Open month dropdown and pick February (index 1，限定到选项，防止 2 月运行时与触发器撞车）
    const monthSelect = screen.getByLabelText('选择月份');
    fireEvent.click(monthSelect);
    fireEvent.click(screen.getByText('Feb', { selector: '[data-dd-value="1"]' }));

    // Click day 15
    const dayButton = screen.getByLabelText('2020-02-15');
    fireEvent.click(dayButton);

    await waitFor(() => {
      expect(onChange).toHaveBeenCalledWith('2020-02-15');
    });
  });

  it('selects date and time for datetime mode', async () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} includeTime />);
    fireEvent.click(screen.getByLabelText('年份输入'));

    fireEvent.click(screen.getByLabelText('选择年份'));
    // 限定到下拉选项（触发器显示当前年份可能与之相同），消除时间依赖
    fireEvent.click(screen.getByText('2021', { selector: '[data-dd-value="2021"]' }));
    fireEvent.click(screen.getByLabelText('选择月份'));
    // data-dd-value=7 为 8 月；触发器显示的当前月份可能与选项文本相同，必须限定到选项
    fireEvent.click(screen.getByText('Aug', { selector: '[data-dd-value="7"]' }));
    fireEvent.click(screen.getByLabelText('2021-08-10'));

    // Time inputs should now be visible
    const hourInput = screen.getByLabelText('小时');
    const minuteInput = screen.getByLabelText('分钟');
    fireEvent.change(hourInput, { target: { value: '8' } });
    fireEvent.change(minuteInput, { target: { value: '5' } });

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith('2021-08-10T08:05');
    });
  });

  it('clears value when clear button is clicked', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2020-02-15" onChange={onChange} />);
    fireEvent.click(screen.getByLabelText('清除'));
    expect(onChange).toHaveBeenCalledWith(undefined);
    // 清空后分段占位符恢复
    expect(screen.getByPlaceholderText('yyyy')).toBeInTheDocument();
  });

  it('closes popover when clicking outside', () => {
    render(<DatePicker onChange={vi.fn()} />);
    fireEvent.click(screen.getByLabelText('年份输入'));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('preserves selected time when changing date in datetime mode', async () => {
    const onChange = vi.fn();
    render(<DatePicker value="2020-01-01T12:30" onChange={onChange} includeTime />);
    fireEvent.click(screen.getByLabelText('年份输入'));

    fireEvent.click(screen.getByLabelText('选择年份'));
    fireEvent.click(screen.getByText('2022'));
    fireEvent.click(screen.getByLabelText('2022-01-01'));

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith('2022-01-01T12:30');
    });
  });

  // ── 直输模式（分段输入） ─────────────────────────────

  it('types a date directly into segments and commits YYYY-MM-DD', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '03' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '15' } });

    expect(onChange).toHaveBeenLastCalledWith('2024-03-15');
  });

  it('types date and time in datetime mode', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} includeTime />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '03' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '15' } });
    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '08' } });
    fireEvent.change(screen.getByLabelText('分钟输入'), { target: { value: '05' } });

    expect(onChange).toHaveBeenLastCalledWith('2024-03-15T08:05');
  });

  it('does not mark valid year-end dates like 2024-12-31 as invalid', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '12' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '31' } });

    // 12 月 / 31 日均在合法边界内，不得标红
    expect(screen.getByLabelText('月份输入')).not.toHaveAttribute('aria-invalid');
    expect(screen.getByLabelText('日期输入')).not.toHaveAttribute('aria-invalid');
    expect(onChange).toHaveBeenLastCalledWith('2024-12-31');
  });

  it('pads and commits single digits in time segments (no silent drop)', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2026-08-22T00:00" onChange={onChange} includeTime />);

    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2026');

    // 单数字分钟：自动补零提交（T00:05 是合法 ISO），不再静默丢弃
    fireEvent.change(screen.getByLabelText('分钟输入'), { target: { value: '5' } });
    expect(onChange).toHaveBeenLastCalledWith('2026-08-22T00:05');
    // 日期不得被清空
    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2026');
    expect((screen.getByLabelText('月份输入') as HTMLInputElement).value).toBe('08');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('22');

    // 单数字小时：同样补零提交
    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '1' } });
    expect(onChange).toHaveBeenLastCalledWith('2026-08-22T01:05');
    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2026');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('22');

    // 两位输入照常提交
    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '15' } });
    expect(onChange).toHaveBeenLastCalledWith('2026-08-22T15:05');
    // 内部提交保留用户输入原样（time 输入保持用户键入的 '5'，不被归一为 '05'）
    expect((screen.getByLabelText('小时输入') as HTMLInputElement).value).toBe('15');
    expect((screen.getByLabelText('分钟输入') as HTMLInputElement).value).toBe('5');
  });

  it('pads single-digit month/day so 2024-1-5 commits as 2024-01-05 (no silent drop)', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '1' } });
    // 日期未输入仍未完整：传播部分草稿（供保存校验发现），不静默丢弃
    expect(onChange).toHaveBeenLastCalledWith('2024-01');

    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '5' } });
    // 单数字月/日补零后提交合法 ISO——此前 `2024-1-5` 永不提交、保存静默丢值
    expect(onChange).toHaveBeenLastCalledWith('2024-01-05');

    // 内部提交保留用户输入原样（day 显示 '5' 而非被归一为 '05'），连续输入不受干扰
    expect((screen.getByLabelText('月份输入') as HTMLInputElement).value).toBe('1');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('5');
  });

  it('pads single-digit month/day in drafts too (invalid single-digit drafts propagate)', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '13' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '5' } });
    // 非法完整草稿补零后提交，保存时校验可在对应字段下报错
    expect(onChange).toHaveBeenLastCalledWith('2024-13-05');
  });

  it('propagates incomplete input as drafts and commits complete-but-invalid dates as drafts', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    // 只填年份：未输满但已有内容 → 传播草稿，保存时校验可发现「填了但没输完」
    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    expect(onChange).toHaveBeenLastCalledWith('2024');

    // 13 月非法但已输满：草稿提交给父组件，保存时校验可在对应字段下报错；分段仍 inline 标红
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '13' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '15' } });
    expect(onChange).toHaveBeenLastCalledWith('2024-13-15');
    expect(screen.getByLabelText('月份输入')).toHaveAttribute('aria-invalid', 'true');

    // 修正为合法月份后提交合法值
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '04' } });
    expect(onChange).toHaveBeenLastCalledWith('2024-04-15');
  });

  it('propagates a partial-year input like 12-12-12 as a draft (no silent drop)', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    // 用户输入「12-12-12」：年份不足 4 位，此前值永不提交、保存静默丢值。
    // 现在传播为草稿，保存时校验能识别非法并在对应字段下报错。
    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '12' } });
    expect(onChange).toHaveBeenLastCalledWith('12');
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '12' } });
    expect(onChange).toHaveBeenLastCalledWith('12-12');
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '12' } });
    expect(onChange).toHaveBeenLastCalledWith('12-12-12');
  });

  it('commits impossible days like Feb 30 as draft so save-time validation can flag them', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '02' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '30' } });

    expect(onChange).toHaveBeenLastCalledWith('2024-02-30');
    expect(screen.getByLabelText('日期输入')).toHaveAttribute('aria-invalid', 'true');
  });

  it('commits complete-but-invalid datetime drafts (impossible date + out-of-range time)', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} includeTime />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '02' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '30' } });
    // 时分未输满：传播部分草稿（含日期部分），不静默丢弃
    expect(onChange).toHaveBeenLastCalledWith('2024-02-30');

    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '25' } });
    fireEvent.change(screen.getByLabelText('分钟输入'), { target: { value: '99' } });

    expect(onChange).toHaveBeenLastCalledWith('2024-02-30T25:99');
    expect(screen.getByLabelText('小时输入')).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByLabelText('分钟输入')).toHaveAttribute('aria-invalid', 'true');
  });

  it('clears a propagated invalid draft when segments are partially cleared (empty field must not error)', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '02' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '30' } });
    expect(onChange).toHaveBeenLastCalledWith('2024-02-30');

    // 用户删掉年份（分段回到未输满）：非法草稿应被撤销，保存时不再对空字段误报错
    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '' } });
    expect(onChange).toHaveBeenLastCalledWith(undefined);
  });

  it('restores the previous valid value when a draft is partially cleared after editing', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2024-02-29" onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '30' } });
    expect(onChange).toHaveBeenLastCalledWith('2024-02-30');

    // 用户删掉日期分段：撤销草稿，恢复原合法值（而非清空或残留草稿）
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '' } });
    expect(onChange).toHaveBeenLastCalledWith('2024-02-29');
  });

  it('displays an unparseable stored value (impossible date) in segments instead of hiding it', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2024-02-30T10:30" onChange={onChange} includeTime />);

    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2024');
    expect((screen.getByLabelText('月份输入') as HTMLInputElement).value).toBe('02');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('30');
    expect((screen.getByLabelText('小时输入') as HTMLInputElement).value).toBe('10');
    expect((screen.getByLabelText('分钟输入') as HTMLInputElement).value).toBe('30');
    // 不可能日期分段标红，用户可看到并修正
    expect(screen.getByLabelText('日期输入')).toHaveAttribute('aria-invalid', 'true');
  });

  it('clears a stored unparseable value when segments are cleared', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2024-02-30T10:30" onChange={onChange} includeTime />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '' } });
    // 存量不可解析值按草稿语义撤销：清空分段即清空父值
    expect(onChange).toHaveBeenLastCalledWith(undefined);
  });

  it('calendar selection overwrites directly typed value', async () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '03' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '15' } });
    expect(onChange).toHaveBeenLastCalledWith('2024-03-15');

    // 打开日历选 2020-02-15，应覆盖输入值（直输自动跳焦已弹出日历卡片；
    // 未弹出时点击输入区域兜底）
    if (!screen.queryByRole('dialog')) {
      fireEvent.click(screen.getByLabelText('年份输入'));
    }
    fireEvent.click(screen.getByLabelText('选择年份'));
    fireEvent.click(screen.getByText('2020', { selector: '[data-dd-value="2020"]' }));
    fireEvent.click(screen.getByLabelText('选择月份'));
    fireEvent.click(screen.getByText('Feb', { selector: '[data-dd-value="1"]' }));
    fireEvent.click(screen.getByLabelText('2020-02-15'));

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith('2020-02-15');
    });
    // 分段显示同步为日历选中的值
    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2020');
    expect((screen.getByLabelText('月份输入') as HTMLInputElement).value).toBe('02');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('15');
  });

  it('shows input hint when a segment is focused', () => {
    render(<DatePicker onChange={vi.fn()} />);
    fireEvent.focus(screen.getByLabelText('年份输入'));
    expect(screen.getByText(/可直接输入数字/)).toBeInTheDocument();
    fireEvent.blur(screen.getByLabelText('年份输入'));
    expect(screen.queryByText(/可直接输入数字/)).not.toBeInTheDocument();
  });

  it('moves focus to next segment when a segment is filled', () => {
    render(<DatePicker onChange={vi.fn()} />);
    const year = screen.getByLabelText('年份输入') as HTMLInputElement;
    fireEvent.change(year, { target: { value: '2024' } });
    expect(document.activeElement).toBe(screen.getByLabelText('月份输入'));
  });

  it('backspace on empty segment moves to previous segment', () => {
    render(<DatePicker onChange={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    expect(document.activeElement).toBe(screen.getByLabelText('月份输入'));
    fireEvent.keyDown(screen.getByLabelText('月份输入'), { key: 'Backspace' });
    expect(document.activeElement).toBe(screen.getByLabelText('年份输入'));
  });

  it('strips non-digit characters from segment input', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);
    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '20a24b' } });
    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2024');
  });

  it('rejects disabled interaction', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2020-02-15" onChange={onChange} disabled />);
    expect(screen.getByLabelText('年份输入')).toBeDisabled();
    expect(screen.queryByLabelText('清除')).not.toBeInTheDocument();
  });
});
