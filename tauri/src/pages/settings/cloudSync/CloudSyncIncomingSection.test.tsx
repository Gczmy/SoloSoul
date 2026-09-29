import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { CloudSyncIncomingSection } from './CloudSyncIncomingSection';

describe('CloudSyncIncomingSection', () => {
  it('shows only snapshot file names for Windows and Unix paths while importing the original path', () => {
    const windowsPath = 'C:\\Users\\Alice\\incoming\\snapshot-a.solosoul';
    const unixPath = '/vault/incoming/snapshot-b.solosoul';
    const onImport = vi.fn();
    render(
      <CloudSyncIncomingSection
        incomingFiles={[windowsPath, unixPath]}
        importingFile={null}
        onImport={onImport}
      />,
    );

    expect(screen.getByText('snapshot-a.solosoul')).toBeInTheDocument();
    expect(screen.getByText('snapshot-b.solosoul')).toBeInTheDocument();
    expect(screen.queryByText(windowsPath)).toBeNull();

    const importButtons = screen.getAllByRole('button', { name: 'settings:cloud_sync_import' });
    fireEvent.click(importButtons[0]);
    fireEvent.click(importButtons[1]);
    expect(onImport).toHaveBeenNthCalledWith(1, windowsPath);
    expect(onImport).toHaveBeenNthCalledWith(2, unixPath);
  });
});
