import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Card } from './Card';

describe('Card keyboard activation', () => {
  it.each(['Enter', ' '])('activates the focused card with %j without a default action', (key) => {
    const onOpen = vi.fn();
    render(<Card onClick={onOpen}>Open record</Card>);
    const card = screen.getByRole('button', { name: 'Open record' });
    card.focus();

    expect(fireEvent.keyDown(card, { key })).toBe(false);
    expect(onOpen).toHaveBeenCalledOnce();
  });

  it.each(['Enter', ' '])('leaves a nested button in control of %j', (key) => {
    const onOpen = vi.fn();
    const onEdit = vi.fn();
    render(
      <Card onClick={onOpen}>
        Record
        <div onClick={(event) => event.stopPropagation()}>
          <button onClick={onEdit}>Edit record</button>
        </div>
      </Card>,
    );
    const button = screen.getByRole('button', { name: 'Edit record' });
    button.focus();

    // Native controls must retain their own default keyboard action. jsdom does
    // not synthesize its click, so dispatch the resulting click explicitly.
    expect(fireEvent.keyDown(button, { key })).toBe(true);
    expect(onOpen).not.toHaveBeenCalled();
    fireEvent.click(button);
    expect(onEdit).toHaveBeenCalledOnce();
    expect(onOpen).not.toHaveBeenCalled();
  });

  it.each(['Enter', ' '])('does not open the card while typing %j in a nested input', (key) => {
    const onOpen = vi.fn();
    render(
      <Card onClick={onOpen}>
        <input aria-label="Record name" />
      </Card>,
    );
    const input = screen.getByRole('textbox', { name: 'Record name' });
    input.focus();

    expect(fireEvent.keyDown(input, { key })).toBe(true);
    expect(onOpen).not.toHaveBeenCalled();
  });

  it('preserves pointer activation and leaves navigation keys alone', () => {
    const onOpen = vi.fn();
    render(<Card onClick={onOpen}>Open record</Card>);
    const card = screen.getByRole('button', { name: 'Open record' });

    expect(fireEvent.keyDown(card, { key: 'Tab' })).toBe(true);
    expect(onOpen).not.toHaveBeenCalled();
    fireEvent.click(card);
    expect(onOpen).toHaveBeenCalledOnce();
  });
});
