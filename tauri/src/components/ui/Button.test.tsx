import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Button } from './Button';

describe('Button', () => {
  it('renders with default variant and size', () => {
    render(<Button>Click me</Button>);
    const button = screen.getByRole('button', { name: /click me/i });
    expect(button).toBeInTheDocument();
    expect(button.className).toMatch(/button/);
    expect(button).toHaveAttribute('data-ui-button', 'primary');
    expect(button).toHaveAttribute('data-ui-intent', 'primary');
    expect(button).toHaveAttribute('data-ui-size', 'md');
  });

  it('renders with different variants', () => {
    const { rerender } = render(<Button variant="secondary">Secondary</Button>);
    for (const [variant, intent] of [
      ['secondary', 'neutral'],
      ['tertiary', 'text'],
      ['glass', 'quiet'],
      ['danger', 'danger'],
      ['danger-outline', 'danger-soft'],
      ['warning', 'warning'],
    ] as const) {
      rerender(<Button variant={variant}>{variant}</Button>);
      expect(screen.getByRole('button')).toHaveAttribute('data-ui-button', variant);
      expect(screen.getByRole('button')).toHaveAttribute('data-ui-intent', intent);
    }
  });

  it('renders with different sizes', () => {
    const { rerender } = render(<Button size="sm">Small</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('data-ui-size', 'sm');

    rerender(<Button size="lg">Large</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('data-ui-size', 'lg');
  });

  it('disables button when loading', () => {
    render(<Button loading>Loading</Button>);
    const button = screen.getByRole('button');
    expect(button).toBeDisabled();
  });

  it('is disabled when disabled prop is true', () => {
    render(<Button disabled>Disabled</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('calls onClick when clicked', () => {
    const handleClick = vi.fn();
    render(<Button onClick={handleClick}>Click</Button>);
    fireEvent.click(screen.getByRole('button'));
    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it('does not call onClick when disabled', () => {
    const handleClick = vi.fn();
    render(
      <Button onClick={handleClick} disabled>
        Click
      </Button>,
    );
    fireEvent.click(screen.getByRole('button'));
    expect(handleClick).not.toHaveBeenCalled();
  });

  it('does not call onClick when loading', () => {
    const handleClick = vi.fn();
    render(
      <Button onClick={handleClick} loading>
        Click
      </Button>,
    );
    fireEvent.click(screen.getByRole('button'));
    expect(handleClick).not.toHaveBeenCalled();
  });

  it('forwards ref correctly', () => {
    const ref = { current: null as HTMLButtonElement | null };
    render(<Button ref={ref}>Ref</Button>);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
  });

  it('applies custom className', () => {
    render(<Button className="custom-class">Custom</Button>);
    expect(screen.getByRole('button')).toHaveClass('custom-class');
  });
});
