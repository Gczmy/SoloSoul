import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SampleTemplateGallery } from './SampleTemplateGallery';
import { SAMPLE_TEMPLATES_EN, SAMPLE_TEMPLATES_ZH } from '@/lib/sampleTemplates';

describe('SampleTemplateGallery', () => {
  it('does not render when closed', () => {
    const { container } = render(
      <SampleTemplateGallery isOpen={false} onClose={vi.fn()} onSelect={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders the English sample template cards by default', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(SAMPLE_TEMPLATES_EN.length);
    SAMPLE_TEMPLATES_EN.forEach((tpl) => {
      expect(screen.getByText(tpl.name)).toBeInTheDocument();
    });
  });

  it('calls onSelect with the chosen template', () => {
    const onSelect = vi.fn();
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={onSelect} />);

    const cards = screen.getAllByTestId('sample-template-card');
    fireEvent.click(cards[0]);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith(SAMPLE_TEMPLATES_EN[0]);
  });

  it('calls onClose when clicking the close button', () => {
    const onClose = vi.fn();
    render(<SampleTemplateGallery isOpen={true} onClose={onClose} onSelect={vi.fn()} />);

    fireEvent.click(screen.getByTestId('sample-gallery-close'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when clicking the overlay', () => {
    const onClose = vi.fn();
    const { container } = render(
      <SampleTemplateGallery isOpen={true} onClose={onClose} onSelect={vi.fn()} />,
    );

    const overlay = container.firstChild as HTMLElement;
    fireEvent.click(overlay);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('filters sample templates by page category', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const travelCount = SAMPLE_TEMPLATES_EN.filter((t) => t.category === 'travel').length;
    fireEvent.click(screen.getByTestId('page-filter-travel'));
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(travelCount);
  });

  it('filters sample templates by search query', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: SAMPLE_TEMPLATES_EN[0].name } });
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(1);
    expect(screen.getByText(SAMPLE_TEMPLATES_EN[0].name)).toBeInTheDocument();
  });

  it('switches to the Chinese tab and resets filters', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    // Apply a category filter first.
    fireEvent.click(screen.getByTestId('page-filter-travel'));
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(
      SAMPLE_TEMPLATES_EN.filter((t) => t.category === 'travel').length,
    );

    // Switch locale tab.
    fireEvent.click(screen.getByTestId('locale-tab-zh'));

    // Should show all Chinese templates with filter reset.
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(SAMPLE_TEMPLATES_ZH.length);
    SAMPLE_TEMPLATES_ZH.forEach((tpl) => {
      expect(screen.getByText(tpl.name)).toBeInTheDocument();
    });
  });

  it('resets search query when switching locale tabs', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: SAMPLE_TEMPLATES_EN[0].name } });
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(1);

    fireEvent.click(screen.getByTestId('locale-tab-zh'));
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(SAMPLE_TEMPLATES_ZH.length);
    expect(input).toHaveValue('');
  });
});
