import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { loadGuideContent, loadGuideIndex, searchGuides } from '@/lib/guideApi';
import type { GuideContent } from '@/lib/guideApi';
import { useShellConfigStore } from '@/components/layout/shellConfigStore';
import { HelpPage } from './HelpPage';

vi.mock('@/lib/guideApi', () => ({
  loadGuideIndex: vi.fn(),
  loadGuideContent: vi.fn(),
  searchGuides: vi.fn(),
}));

describe('HelpPage document ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(loadGuideContent).mockReset();
    vi.mocked(loadGuideIndex).mockResolvedValue({ guides: [], categories: [] });
    vi.mocked(searchGuides).mockResolvedValue([]);
  });

  it('retries failed document content rather than only reloading the index', async () => {
    vi.mocked(loadGuideContent)
      .mockRejectedValueOnce(new Error('temporary content failure'))
      .mockResolvedValueOnce({ id: 'guide', title: 'Recovered guide', content: 'Recovered body' });
    const router = createMemoryRouter([{ path: '/help', element: <HelpPage /> }], {
      initialEntries: ['/help?id=guide'],
    });
    render(<RouterProvider router={router} />);

    expect(await screen.findByText('无法加载文档内容')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '重试' }));

    expect(await screen.findByText('Recovered body')).toBeInTheDocument();
    expect(loadGuideContent).toHaveBeenCalledTimes(2);
    expect(loadGuideContent).toHaveBeenNthCalledWith(
      2,
      'guide',
      vi.mocked(loadGuideContent).mock.calls[0][1],
    );
  });

  it('still retries the index when the index request fails', async () => {
    vi.mocked(loadGuideIndex).mockRejectedValueOnce(new Error('index unavailable'));
    const router = createMemoryRouter([{ path: '/help', element: <HelpPage /> }], {
      initialEntries: ['/help'],
    });
    render(<RouterProvider router={router} />);

    expect(await screen.findByText('无法加载帮助索引')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '重试' }));

    await waitFor(() => expect(loadGuideIndex).toHaveBeenCalledTimes(2));
    expect(loadGuideContent).not.toHaveBeenCalled();
    expect(screen.queryByText('无法加载帮助索引')).not.toBeInTheDocument();
  });

  it('does not show a previous document after returning to the index or another document fails', async () => {
    let rejectSecond!: (reason?: unknown) => void;
    vi.mocked(loadGuideContent).mockImplementation((id) =>
      id === 'first'
        ? Promise.resolve({ id, title: 'First guide', content: 'First guide body' })
        : new Promise<GuideContent>((_, reject) => {
            rejectSecond = reject;
          }),
    );
    const router = createMemoryRouter([{ path: '/help', element: <HelpPage /> }], {
      initialEntries: ['/help?id=first'],
    });
    render(<RouterProvider router={router} />);

    expect(await screen.findByText('First guide body')).toBeInTheDocument();
    expect(useShellConfigStore.getState().title).toBe('First guide');

    await act(async () => {
      await router.navigate('/help?id=second');
    });
    expect(screen.queryByText('First guide body')).not.toBeInTheDocument();
    expect(useShellConfigStore.getState().title).toBe('settings:items.help_docs');
    await act(async () => {
      rejectSecond(new Error('Second guide failed'));
    });
    expect(await screen.findByText('无法加载文档内容')).toBeInTheDocument();
    expect(screen.queryByText('First guide body')).not.toBeInTheDocument();
    expect(useShellConfigStore.getState().title).toBe('settings:items.help_docs');

    await act(async () => {
      await router.navigate('/help');
    });
    await waitFor(() => {
      expect(useShellConfigStore.getState().title).toBe('settings:items.help_docs');
    });
    expect(screen.queryByText('First guide body')).not.toBeInTheDocument();
    expect(screen.queryByText('无法加载文档内容')).not.toBeInTheDocument();
  });
});
