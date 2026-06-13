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

    SAMPLE_TEMPLATES.forEach((tpl) => {
      expect(screen.getByText(tpl.nameI18nKey)).toBeInTheDocument();
    });
    expect(screen.getAllByRole('button')).toHaveLength(SAMPLE_TEMPLATES.length + 1); // cards + close
  });

  it('calls onSelect with the chosen template', () => {
    const onSelect = vi.fn();
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={onSelect} />);

    const firstCard = screen.getByText(SAMPLE_TEMPLATES[0].nameI18nKey);
    fireEvent.click(firstCard);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith(SAMPLE_TEMPLATES[0]);
  });

  it('calls onClose when clicking the close button', () => {
    const onClose = vi.fn();
    render(<SampleTemplateGallery isOpen={true} onClose={onClose} onSelect={vi.fn()} />);

    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBe(SAMPLE_TEMPLATES.length + 1);
    fireEvent.click(buttons[0]);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when clicking the overlay', () => {
    const onClose = vi.fn();
    const { container } = render(
      <SampleTemplateGallery isOpen={true} onClose={onClose} onSelect={vi.fn()} />,
    );

    // The overlay is the outermost div
    const overlay = container.firstChild as HTMLElement;
    fireEvent.click(overlay);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
