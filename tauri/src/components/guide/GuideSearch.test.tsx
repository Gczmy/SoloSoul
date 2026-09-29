import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import type { GuideContent } from '@/lib/guideApi';
import { GuideSearch } from './GuideSearch';

describe('GuideSearch request ownership', () => {
  afterEach(() => vi.useRealTimers());

  it('keeps the newest results and ignores a response after the query is cleared', async () => {
    vi.useFakeTimers();
    const pending = new Map<string, (results: GuideContent[]) => void>();
    const onSearch = vi.fn(
      (query: string) =>
        new Promise<GuideContent[]>((resolve) => {
          pending.set(query, resolve);
        }),
    );
    render(<GuideSearch language="en" onSearch={onSearch} onSelect={vi.fn()} />);
    const input = screen.getByPlaceholderText('search_help_docs');

    fireEvent.change(input, { target: { value: 'old-rf935-query' } });
    await act(async () => vi.advanceTimersByTime(DEBOUNCE_DELAY_MS));
    fireEvent.change(input, { target: { value: 'new-rf935-query' } });
    await act(async () => vi.advanceTimersByTime(DEBOUNCE_DELAY_MS));
    expect(onSearch).toHaveBeenCalledTimes(2);

    await act(async () => {
      pending.get('new-rf935-query')!([
        { id: 'new', title: 'New guide', content: 'Current result' },
      ]);
    });
    expect(screen.getByRole('button', { name: /New\s*guide/ })).toBeInTheDocument();
    await act(async () => {
      pending.get('old-rf935-query')!([{ id: 'old', title: 'Old guide', content: 'Stale result' }]);
    });
    expect(screen.getByRole('button', { name: /New\s*guide/ })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Old\s*guide/ })).not.toBeInTheDocument();

    fireEvent.change(input, { target: { value: 'clear-rf935-query' } });
    await act(async () => vi.advanceTimersByTime(DEBOUNCE_DELAY_MS));
    fireEvent.click(screen.getByRole('button', { name: 'Clear' }));
    await act(async () => {
      pending.get('clear-rf935-query')!([
        { id: 'cleared', title: 'Cleared guide', content: 'Result after clear' },
      ]);
    });
    expect(screen.queryByRole('button', { name: /Cleared\s*guide/ })).not.toBeInTheDocument();
    expect(screen.queryByText('searching')).not.toBeInTheDocument();
  });

  it('does not reuse a previous language result for the same query', async () => {
    vi.useFakeTimers();
    const enSearch = vi
      .fn()
      .mockResolvedValue([{ id: 'guide', title: 'English manual', content: 'English content' }]);
    const zhSearch = vi
      .fn()
      .mockResolvedValue([{ id: 'guide', title: '中文指南', content: '中文内容' }]);
    const onSelect = vi.fn();
    const query = 'rf936-language-query';
    const { rerender } = render(
      <GuideSearch key="en" language="en" onSearch={enSearch} onSelect={onSelect} />,
    );
    fireEvent.change(screen.getByPlaceholderText('search_help_docs'), { target: { value: query } });
    await act(async () => vi.advanceTimersByTime(DEBOUNCE_DELAY_MS));
    expect(screen.getByRole('button', { name: /English manual/ })).toBeInTheDocument();

    rerender(<GuideSearch key="zh" language="zh" onSearch={zhSearch} onSelect={onSelect} />);
    fireEvent.change(screen.getByPlaceholderText('search_help_docs'), { target: { value: query } });
    await act(async () => vi.advanceTimersByTime(DEBOUNCE_DELAY_MS));
    expect(zhSearch).toHaveBeenCalledWith(query);
    expect(screen.getByRole('button', { name: /中文指南/ })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /English manual/ })).not.toBeInTheDocument();
  });
});
