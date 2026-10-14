import { act, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, expect, test, vi } from 'vitest';
import { ToolbarActions } from './ToolbarActions';
import { resizeObserverInstances } from '@/test/setup';

afterEach(() => {
  vi.restoreAllMocks();
});

test('wide action groups remain collected after restore until the whole row fits again', () => {
  let width = 900;
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (
    this: HTMLElement,
  ) {
    return { width: this.dataset.compact === 'false' ? 716 : 170 } as DOMRect;
  });
  const { container } = render(
    <MemoryRouter>
      <header
        ref={(element) => {
          if (element) Object.defineProperty(element, 'clientWidth', { get: () => width });
        }}
        style={{ paddingLeft: 92, paddingRight: 158 }}
      >
        <ToolbarActions primary={<button>Primary</button>}>
          <button>Secondary</button>
        </ToolbarActions>
      </header>
    </MemoryRouter>,
  );
  const toolbar = container.querySelector('[data-compact]')!;
  expect(toolbar).toHaveAttribute('data-compact', 'true');
  const resize = () => act(() => resizeObserverInstances.at(-1)!.trigger());
  resize();
  expect(toolbar).toHaveAttribute('data-compact', 'true');
  width = 1440;
  resize();
  expect(toolbar).toHaveAttribute('data-compact', 'false');
  width = 900;
  resize();
  expect(toolbar).toHaveAttribute('data-compact', 'true');
  expect(screen.getByRole('button', { name: 'Primary' })).toBeVisible();
});
