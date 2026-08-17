import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { RouteLoadingSkeleton } from './RouteLoadingSkeleton';

describe('RouteLoadingSkeleton', () => {
  it('renders the route-level skeleton placeholder', () => {
    render(<RouteLoadingSkeleton />);
    expect(screen.getByTestId('route-loading-skeleton')).toBeInTheDocument();
  });

  it('is decorative (aria-hidden, no focusable elements)', () => {
    const { container } = render(<RouteLoadingSkeleton />);
    expect(container.querySelector('[aria-hidden="true"]')).not.toBeNull();
    expect(container.querySelector('button, a, input, [tabindex]')).toBeNull();
  });
});
