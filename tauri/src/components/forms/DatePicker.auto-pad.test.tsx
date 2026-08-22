import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { DatePicker } from './DatePicker';

describe('DatePicker single-digit segment behavior', () => {
  it('month segment stays as "1" (not "01") while user is still typing day', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    // 填年份 4 位
    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '1212' } });
    // 填月份 1 位
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '1' } });

    // 月份分段应保持用户输入的原始值 "1"，而不是被自动补零为 "01"
    expect((screen.getByLabelText('月份输入') as HTMLInputElement).value).toBe('1');
  });

  it('month segment pads to "01" only AFTER all segments are complete and committed', () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('年份输入'), { target: { value: '1212' } });
    fireEvent.change(screen.getByLabelText('月份输入'), { target: { value: '1' } });
    fireEvent.change(screen.getByLabelText('日期输入'), { target: { value: '5' } });

    // 三个分段都填完后，提交补零值
    expect(onChange).toHaveBeenLastCalledWith('1212-01-05');
    // 内部提交保留用户输入原样（不会出现 1→01 的自动补零干扰）
    expect((screen.getByLabelText('月份输入') as HTMLInputElement).value).toBe('1');
    expect((screen.getByLabelText('日期输入') as HTMLInputElement).value).toBe('5');
  });
});
