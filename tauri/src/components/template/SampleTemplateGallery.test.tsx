import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SampleTemplateGallery } from './SampleTemplateGallery';
import { SAMPLE_TEMPLATES } from '@/lib/sampleTemplates';

describe('SampleTemplateGallery', () => {
  it('does not render when closed', () => {
    const { container } = render(
      <SampleTemplateGallery isOpen={false} onClose={vi.fn()} onSelect={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders all sample template cards when open', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(SAMPLE_TEMPLATES.length);
    SAMPLE_TEMPLATES.forEach((tpl) => {
      expect(screen.getByText(tpl.nameI18nKey)).toBeInTheDocument();
    });
  });

  it('calls onSelect with the chosen template', () => {
    const onSelect = vi.fn();
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={onSelect} />);

    const cards = screen.getAllByTestId('sample-template-card');
    fireEvent.click(cards[0]);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith(SAMPLE_TEMPLATES[0]);
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

    const travelCount = SAMPLE_TEMPLATES.filter((t) => t.category === 'travel').length;
    fireEvent.click(screen.getByTestId('page-filter-travel'));
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(travelCount);
  });

  it('filters sample templates by search query', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: SAMPLE_TEMPLATES[0].nameI18nKey } });
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(1);
    expect(screen.getByText(SAMPLE_TEMPLATES[0].nameI18nKey)).toBeInTheDocument();
  });
});
