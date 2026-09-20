import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ObjectSelectionTree } from './ObjectSelectionTree';

function setup() {
  const callbacks = {
    onTogglePage: vi.fn(),
    onToggleObject: vi.fn(),
    onToggleObjectExpanded: vi.fn(),
    onToggleAttachment: vi.fn(),
    onToggleExpandedPage: vi.fn(),
    onSelectAll: vi.fn(),
  };
  render(
    <ObjectSelectionTree
      pageGroups={[
        {
          sectionType: 'identity',
          objects: [
            { id: 'obj-1', name: 'Passport', sectionType: 'identity', sensitivityLevel: 'public' },
          ],
        },
      ]}
      selectedPageIds={new Set()}
      expandedPages={new Set(['identity'])}
      expandedObjects={new Set(['obj-1'])}
      selectedAttachmentIds={new Set()}
      objectAttachments={
        new Map([['obj-1', [{ id: 'att-1', fileName: 'passport.pdf', sizeBytes: 1024 }]]])
      }
      totalSelected={0}
      showAttachmentExpand={() => true}
      isObjectSelected={() => false}
      {...callbacks}
    />,
  );
  return callbacks;
}

describe('ObjectSelectionTree 选择交互', () => {
  it('全选框和外层全选行各只触发一次，避免 checkbox 与父行重复选择', () => {
    const { onSelectAll } = setup();
    fireEvent.click(screen.getByRole('checkbox', { name: 'common:select_all' }));
    expect(onSelectAll).toHaveBeenCalledExactlyOnceWith(true);
    fireEvent.click(screen.getByText('common:select_all'));
    expect(onSelectAll).toHaveBeenCalledTimes(2);
  });

  it('页面复选框只选择，不展开页面', () => {
    const { onTogglePage, onToggleExpandedPage } = setup();
    fireEvent.click(screen.getByRole('checkbox', { name: 'navigation:identity' }));
    expect(onTogglePage).toHaveBeenCalledExactlyOnceWith('identity', ['obj-1']);
    expect(onToggleExpandedPage).not.toHaveBeenCalled();
  });

  it('对象与附件 label 文本均驱动各自复选框', () => {
    const { onToggleObject, onToggleAttachment } = setup();
    fireEvent.click(screen.getByText('Passport'));
    expect(onToggleObject).toHaveBeenCalledExactlyOnceWith('obj-1', 'identity', ['obj-1']);
    fireEvent.click(screen.getByText('passport.pdf'));
    expect(onToggleAttachment).toHaveBeenCalledExactlyOnceWith('att-1', 'obj-1', 'identity', [
      'obj-1',
    ]);
  });
});
