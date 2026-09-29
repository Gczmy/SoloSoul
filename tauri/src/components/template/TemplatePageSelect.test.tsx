import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';
import { TemplatePageSelect } from './TemplatePageSelect';

const originalCustomPages = useSettingsStore.getState().settings.customPages;

function setCustomPages(pages: CustomPage[]) {
  act(() => {
    useSettingsStore.setState((state) => ({
      settings: { ...state.settings, customPages: pages },
    }));
  });
}

const activePage: CustomPage = {
  id: 'active-page',
  name: 'Active Page',
  iconId: 'file-text',
  createdAt: '2026-09-01T00:00:00Z',
  sortOrder: 0,
};
const deletedPage: CustomPage = {
  ...activePage,
  id: 'deleted-page',
  name: 'Deleted Page',
  deletedAt: '2026-09-02T00:00:00Z',
  sortOrder: 1,
};

describe('TemplatePageSelect', () => {
  afterEach(() => {
    setCustomPages(originalCustomPages);
  });

  it('associates its label and offers system and active custom pages', () => {
    setCustomPages([activePage, deletedPage]);
    const onChange = vi.fn();
    render(<TemplatePageSelect value="identity" label="所属页面" onChange={onChange} />);

    const select = screen.getByRole('combobox', { name: '所属页面' });
    expect(select).toHaveValue('identity');
    expect(screen.getAllByRole('option')).toHaveLength(6);
    expect(screen.getByRole('option', { name: 'Active Page' })).not.toBeDisabled();
    expect(screen.getByRole('option', { name: 'Deleted Page' })).toBeDisabled();

    fireEvent.change(select, { target: { value: 'active-page' } });
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith('active-page');
  });

  it('keeps removed page values visible as a disabled fallback', () => {
    setCustomPages([]);
    render(<TemplatePageSelect value="missing-page" onChange={vi.fn()} />);
    const select = screen.getByRole('combobox');
    expect(select).toHaveValue('missing-page');
    const fallback = screen.getAllByRole('option').at(-1);
    expect(fallback).toHaveValue('missing-page');
    expect(fallback).toBeDisabled();
  });

  it('does not offer an unknown fallback for a known deleted page', () => {
    setCustomPages([deletedPage]);
    render(<TemplatePageSelect value="deleted-page" onChange={vi.fn()} />);
    expect(screen.getByRole('combobox')).toHaveValue('deleted-page');
    expect(screen.getAllByRole('option')).toHaveLength(5);
    expect(screen.getByRole('option', { name: 'Deleted Page' })).toBeDisabled();
  });
});
