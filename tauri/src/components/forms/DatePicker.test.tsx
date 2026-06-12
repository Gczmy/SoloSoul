import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DatePicker } from './DatePicker';

describe('DatePicker', () => {
  it('renders placeholder when no value is provided', () => {
    render(<DatePicker onChange={vi.fn()} />);
    expect(screen.getByText('YYYY-MM-DD')).toBeInTheDocument();
  });

  it('opens calendar when trigger is clicked', () => {
    render(<DatePicker onChange={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { expanded: false }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('selects a date and calls onChange with YYYY-MM-DD', async () => {
    const onChange = vi.fn();
    render(<DatePicker onChange={onChange} />);
    fireEvent.click(screen.getByRole('button', { expanded: false }));

    // Open year dropdown and pick 2020
    const yearSelect = screen.getByLabelText('年份');
    fireEvent.change(yearSelect, { target: { value: '2020' } });

    // Open month dropdown and pick February (1)
    const monthSelect = screen.getByLabelText('月份');
    fireEvent.change(monthSelect, { target: { value: '1' } });

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
    fireEvent.click(screen.getByRole('button', { expanded: false }));

    fireEvent.change(screen.getByLabelText('年份'), { target: { value: '2021' } });
    fireEvent.change(screen.getByLabelText('月份'), { target: { value: '5' } });
    fireEvent.click(screen.getByLabelText('2021-06-10'));

    // Time inputs should now be visible
    const hourInput = screen.getByLabelText('小时');
    const minuteInput = screen.getByLabelText('分钟');
    fireEvent.change(hourInput, { target: { value: '8' } });
    fireEvent.change(minuteInput, { target: { value: '5' } });

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith("2021-06-10T08:05");
    });
  });

  it('clears value when clear button is clicked', () => {
    const onChange = vi.fn();
    render(<DatePicker value="2020-02-15" onChange={onChange} />);
    fireEvent.click(screen.getByLabelText('清除'));
    expect(onChange).toHaveBeenCalledWith(undefined);
  });

  it('closes popover when clicking outside', () => {
    render(<DatePicker onChange={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { expanded: false }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('preserves selected time when changing date in datetime mode', async () => {
    const onChange = vi.fn();
    render(<DatePicker value="2020-01-01T12:30" onChange={onChange} includeTime />);
    fireEvent.click(screen.getByRole('button', { expanded: false }));

    fireEvent.change(screen.getByLabelText('年份'), { target: { value: '2022' } });
    fireEvent.click(screen.getByLabelText('2022-01-01'));

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith('2022-01-01T12:30');
    });
  });
});
