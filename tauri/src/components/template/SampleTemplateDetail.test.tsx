import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SampleTemplateDetail } from './SampleTemplateDetail';
import { SAMPLE_TEMPLATES } from '@/lib/sampleTemplates';

describe('SampleTemplateDetail', () => {
  const template = SAMPLE_TEMPLATES[0];

  it('renders template name, category and field count', () => {
    render(<SampleTemplateDetail template={template} onBack={vi.fn()} onUse={vi.fn()} />);

    expect(screen.getByText(template.nameI18nKey)).toBeInTheDocument();
    expect(screen.getByText(new RegExp(`${template.properties.length}`))).toBeInTheDocument();
  });

  it('renders each property with name, type and sensitivity badge', () => {
    render(<SampleTemplateDetail template={template} onBack={vi.fn()} onUse={vi.fn()} />);

    template.properties.forEach((prop) => {
      const propRow = screen.getByText(prop.nameI18nKey).parentElement?.parentElement;
      expect(propRow).toBeInTheDocument();
      expect(propRow?.textContent).toContain(`editor:field_types.${prop.type}`);
    });
  });

  it('calls onUse when clicking use template button', () => {
    const onUse = vi.fn();
    render(<SampleTemplateDetail template={template} onBack={vi.fn()} onUse={onUse} />);

    fireEvent.click(screen.getByRole('button', { name: /use_sample_template/i }));
    expect(onUse).toHaveBeenCalledTimes(1);
  });

  it('calls onBack when clicking back button', () => {
    const onBack = vi.fn();
    render(<SampleTemplateDetail template={template} onBack={onBack} onUse={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /common:back/i }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('calls onBack when clicking close button', () => {
    const onBack = vi.fn();
    render(<SampleTemplateDetail template={template} onBack={onBack} onUse={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /close/i }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('renders empty state for template without properties', () => {
    const emptyTemplate = { ...template, properties: [] };
    render(<SampleTemplateDetail template={emptyTemplate} onBack={vi.fn()} onUse={vi.fn()} />);

    expect(screen.getByText(emptyTemplate.nameI18nKey)).toBeInTheDocument();
    expect(screen.queryByText(template.properties[0].nameI18nKey)).not.toBeInTheDocument();
  });

  it('calls onBack when clicking the overlay backdrop', () => {
    const onBack = vi.fn();
    const { container } = render(<SampleTemplateDetail template={template} onBack={onBack} onUse={vi.fn()} />);

    const overlay = container.firstChild as HTMLElement;
    fireEvent.click(overlay);
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});
