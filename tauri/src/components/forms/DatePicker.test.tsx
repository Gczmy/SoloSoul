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

  it('keeps the date while typing single digits into time segments', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2026-08-22T00:00" onChange={onChange} includeTime />);

    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2026');

    // 单数字分钟：未输满不应提交（不得产出 T00:5 这类非 ISO 值，否则整行被清空）
    fireEvent.change(screen.getByLabelText('分钟输入'), { target: { value: '5' } });
    expect(onChange).not.toHaveBeenCalled();
    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2026');
    expect((screen.getByLabelText('月份输入') as HTMLInputElement).value).toBe('08');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('22');

    // 单数字小时：同样不提交、不清空日期
    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '1' } });
    expect(onChange).not.toHaveBeenCalled();
    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2026');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('22');

    // 时分均补全为两位后提交为合法格式
    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '15' } });
    expect(onChange).not.toHaveBeenCalled(); // 分钟仍只有一位
    fireEvent.change(screen.getByLabelText('分钟输入'), { target: { value: '05' } });
    expect(onChange).toHaveBeenLastCalledWith('2026-08-22T15:05');
    expect((screen.getByLabelText('年份输入') as HTMLInputElement).value).toBe('2026');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('22');
    expect((screen.getByLabelText('小时输入') as HTMLInputElement).value).toBe('15');
  });

  it('does not commit incomplete or invalid dates', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    // 只填年份：不提交
    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    expect(onChange).not.toHaveBeenCalled();

    // 13 月非法：不提交
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '13' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '15' } });
    expect(onChange).not.toHaveBeenCalled();

    // 非法分段带 aria-invalid 提示
    expect(screen.getByLabelText('月份输入')).toHaveAttribute('aria-invalid', 'true');

    // 修正为合法月份后提交
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '04' } });
    expect(onChange).toHaveBeenLastCalledWith('2024-04-15');
  });

  it('rejects impossible days like Feb 30', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '02' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '30' } });

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByLabelText('日期输入')).toHaveAttribute('aria-invalid', 'true');
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
