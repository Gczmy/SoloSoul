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
    const yearSelect = screen.getByLabelText('选择年份');
    fireEvent.click(yearSelect);
    fireEvent.click(screen.getByText('2020'));

    // Open month dropdown and pick February (1)
    const monthSelect = screen.getByLabelText('选择月份');
    fireEvent.click(monthSelect);
    fireEvent.click(screen.getByText('Feb'));

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

    fireEvent.click(screen.getByLabelText('选择年份'));
    fireEvent.click(screen.getByText('2021'));
    // Use a month whose label doesn't collide with the trigger's current month
    fireEvent.click(screen.getByLabelText('选择月份'));
    fireEvent.click(screen.getByText('Aug'));
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

    fireEvent.click(screen.getByLabelText('选择年份'));
    fireEvent.click(screen.getByText('2022'));
    fireEvent.click(screen.getByLabelText('2022-01-01'));

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith('2022-01-01T12:30');
    });
  });
});
