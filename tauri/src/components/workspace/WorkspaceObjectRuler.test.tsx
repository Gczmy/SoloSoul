import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { ObjectSummary } from '@/stores/objectStore';
import { WorkspaceObjectRuler } from './WorkspaceObjectRuler';

const { navigateTo, scrollToIndex } = vi.hoisted(() => ({
  navigateTo: vi.fn(),
  scrollToIndex: vi.fn(),
}));

vi.mock('./useObjectRulerPosition', () => ({
  useObjectRulerPosition: () => ({ activeId: 'first', navigateTo }),
}));
vi.mock('./useRulerRail', () => ({
  useRulerRail: () => ({ step: 18, scrollToIndex }),
}));

const objects: ObjectSummary[] = [
  { id: 'first', name: '第一件', typeId: 'identity' },
  { id: 'second', name: '第二件', typeId: 'identity' },
  { id: 'third', name: '第三件', typeId: 'identity' },
].map((object) => ({
  ...object,
  sensitivityLevel: 'public',
  createdAt: '',
  updatedAt: '',
  properties: {},
}));

function renderRuler() {
  return render(
    <WorkspaceObjectRuler
      objects={objects}
      renderedCount={objects.length}
      listRef={{ current: null }}
      revealObject={vi.fn()}
      userTemplates={[]}
      resolveCollectionLabel={() => '身份'}
      attachmentCounts={{}}
    />,
  );
}

beforeEach(() => {
  navigateTo.mockClear();
  scrollToIndex.mockClear();
});

it('键盘方向键与 Home/End 移动唯一可 Tab 焦点，Escape 关闭预览', () => {
  renderRuler();
  const ticks = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-ruler-index]'));
  expect(ticks).toHaveLength(3);
  expect(ticks.map((tick) => tick.tabIndex)).toEqual([0, -1, -1]);

  fireEvent.focus(ticks[0]);
  expect(screen.getByRole('region')).toHaveTextContent('第一件');
  fireEvent.keyDown(ticks[0], { key: 'ArrowDown' });
  expect(ticks[1]).toHaveFocus();
  expect(ticks.map((tick) => tick.tabIndex)).toEqual([-1, 0, -1]);
  expect(screen.getByRole('region')).toHaveTextContent('第二件');

  fireEvent.keyDown(ticks[1], { key: 'End' });
  expect(ticks[2]).toHaveFocus();
  fireEvent.keyDown(ticks[2], { key: 'Home' });
  expect(ticks[0]).toHaveFocus();
  fireEvent.keyDown(ticks[0], { key: 'Escape' });
  expect(screen.queryByRole('region')).toBeNull();
  expect(navigateTo).not.toHaveBeenCalled();
});

it('鼠标点击刻度跳转对应对象，键盘激活保留键盘焦点语义', () => {
  renderRuler();
  const ticks = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-ruler-index]'));
  fireEvent.click(ticks[1], { detail: 1 });
  expect(navigateTo).toHaveBeenCalledWith(1, false);
  fireEvent.click(ticks[2], { detail: 0 });
  expect(navigateTo).toHaveBeenLastCalledWith(2, true);
});
