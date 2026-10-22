import { fireEvent, render, screen } from '@testing-library/react';
import { it, expect, vi } from 'vitest';
import { DatePicker } from './DatePicker';
vi.mock('@/lib/platform', () => ({ isAndroidSync: () => true }));

it('Android keeps invalid stored dates visible and explicitly clearable', () => {
  const onChange = vi.fn();
  render(<DatePicker value="2024-02-30" onChange={onChange} />);
  expect(screen.getByRole('button', { name: 'date_picker' })).toHaveTextContent('2024-02-30');
  fireEvent.click(screen.getByRole('button', { name: 'date_picker' }));
  expect(screen.getByLabelText('date_segment_day')).toHaveValue('30');
  expect(screen.getByLabelText('date_segment_day')).toHaveAttribute('aria-invalid', 'true');
  fireEvent.click(screen.getByRole('button', { name: 'clear' }));
  expect(onChange).toHaveBeenCalledExactlyOnceWith(undefined);
});
it('Android preserves incomplete draft propagation and datetime time when picking a date', () => {
  const onChange = vi.fn();
  const { container } = render(
    <DatePicker value="2024-01-01T12:30" includeTime onChange={onChange} />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'date_time_picker' }));
  fireEvent.change(container.ownerDocument.querySelector('input[type=date]')!, {
    target: { value: '2024-02-29' },
  });
  expect(onChange).toHaveBeenLastCalledWith('2024-02-29T12:30');
  fireEvent.change(screen.getByLabelText('date_segment_year'), { target: { value: '12' } });
  expect(onChange).toHaveBeenLastCalledWith('12-02-29T12:30');
});
it('Android disabled date cannot open a picker', () => {
  render(<DatePicker value="2024-01-01" onChange={vi.fn()} disabled />);
  fireEvent.click(screen.getByRole('button', { name: 'date_picker' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
