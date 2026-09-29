import { useState } from 'react';
import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useOcrScanStore, type OcrScanEntry } from '@/stores/ocrScanStore';
import { OcrHistoryTrashDropdown } from './OcrHistoryTrashDropdown';

const onSelectEntry = vi.fn();

function entry(
  id: string,
  isDeleted = false,
  mode: OcrScanEntry['mode'] = 'general',
): OcrScanEntry {
  return {
    id,
    timestamp: Date.UTC(2026, 8, 29),
    filePath: `/scans/${id}.pdf`,
    fileName: `${id}.pdf`,
    mode,
    result: null,
    mrzResult: null,
    isDeleted,
    ...(isDeleted ? { deletedAt: Date.UTC(2026, 8, 29) } : {}),
  };
}

function Menu({ initialTrash = false }: { initialTrash?: boolean }) {
  const [showTrash, setShowTrash] = useState(initialTrash);
  const [currentEntryId, setCurrentEntryId] = useState<string | null>(null);
  const history = useOcrScanStore((state) => state.scanHistory);
  return (
    <OcrHistoryTrashDropdown
      showTrash={showTrash}
      onShowTrashChange={setShowTrash}
      activeHistory={history.filter((item) => !item.isDeleted)}
      trash={history.filter((item) => item.isDeleted)}
      currentEntryId={currentEntryId}
      onSelectEntry={(item) => {
        onSelectEntry(item);
        setCurrentEntryId(item.id);
      }}
    />
  );
}

beforeEach(() => onSelectEntry.mockClear());
afterEach(() => act(() => useOcrScanStore.setState({ scanHistory: [] })));

describe('OCR history and trash menu', () => {
  it('deletes without selecting a scan, then restores it from trash', () => {
    const first = entry('first');
    const second = entry('second', false, 'mrz');
    useOcrScanStore.setState({ scanHistory: [first, second] });
    render(<Menu />);

    fireEvent.click(screen.getByTitle('first.pdf'));
    expect(onSelectEntry).toHaveBeenCalledExactlyOnceWith(first);
    fireEvent.click(
      within(screen.getByTitle('second.pdf')).getByRole('button', { name: 'common:delete' }),
    );
    expect(onSelectEntry).toHaveBeenCalledTimes(1);
    expect(
      useOcrScanStore.getState().scanHistory.find((item) => item.id === second.id)?.isDeleted,
    ).toBe(true);

    fireEvent.click(screen.getByRole('button', { name: 'ocr:trash_tab (1)' }));
    expect(screen.getByText('second.pdf')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'ocr:restore' }));
    expect(
      useOcrScanStore.getState().scanHistory.find((item) => item.id === second.id)?.isDeleted,
    ).toBe(false);
    expect(screen.getByText('ocr:trash_empty')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'ocr:history_tab (2)' }));
    expect(screen.getByTitle('second.pdf')).toBeInTheDocument();
  });

  it('permanently deletes one item and clears the remaining trash', () => {
    useOcrScanStore.setState({ scanHistory: [entry('discard-a', true), entry('discard-b', true)] });
    render(<Menu initialTrash />);

    fireEvent.click(screen.getAllByRole('button', { name: 'ocr:permanently_delete' })[0]);
    expect(useOcrScanStore.getState().scanHistory.map((item) => item.id)).toEqual(['discard-b']);
    expect(screen.queryByRole('button', { name: 'ocr:clear_trash' })).not.toBeInTheDocument();

    act(() => {
      useOcrScanStore.setState({
        scanHistory: [entry('discard-b', true), entry('discard-c', true)],
      });
    });
    fireEvent.click(screen.getByRole('button', { name: 'ocr:clear_trash' }));
    expect(useOcrScanStore.getState().scanHistory).toEqual([]);
    expect(screen.getByText('ocr:trash_empty')).toBeInTheDocument();
  });

  it('shows empty history and trash states without scan actions', () => {
    useOcrScanStore.setState({ scanHistory: [] });
    render(<Menu />);
    expect(screen.getByText('ocr:no_history')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'ocr:trash_tab (0)' }));
    expect(screen.getByText('ocr:trash_empty')).toBeInTheDocument();
    expect(onSelectEntry).not.toHaveBeenCalled();
  });
});
