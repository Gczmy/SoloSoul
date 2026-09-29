import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, it, expect } from 'vitest';
import { useSettingsStore } from '@/stores/settingsStore';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import {
  CUSTOMIZABLE_ACTION_IDS,
  CUSTOMIZABLE_LINKS,
  useBoundNavActions,
  useMobileNavActions,
} from './useNavigationItems';

describe('useNavigationItems constants', () => {
  it('includes ocr in customizable action ids', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toContain('ocr');
  });

  it('does not map ocr to a link because it is a sidebar action', () => {
    expect(CUSTOMIZABLE_LINKS).not.toHaveProperty('ocr');
  });

  it('CUSTOMIZABLE_ACTION_IDS has exactly 10 items', () => {
    expect(CUSTOMIZABLE_ACTION_IDS).toHaveLength(10);
    expect(CUSTOMIZABLE_ACTION_IDS).toEqual([
      'search',
      'trash',
      'templates',
      'attachments',
      'plugins',
      'ocr',
      'import_export',
      'sync',
      'help',
      'ai_chat',
    ]);
  });
});

describe('sidebar and mobile navigation behavior', () => {
  const originalModes = useSettingsStore.getState().settings.sidebarButtonModes;

  beforeEach(() => {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        sidebarButtonModes: { search: 'card', ocr: 'card', plugins: 'card', ai_chat: 'card' },
      },
    });
    useOcrScanStore.setState({ isCardOpen: false });
    usePluginQuickStore.setState({ isOpen: false });
  });

  afterEach(() => {
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, sidebarButtonModes: originalModes },
    });
  });

  it('desktop card actions toggle only their corresponding panel', () => {
    const { result } = renderHook(() => useBoundNavActions());
    const actionFor = (key: string) => {
      const item = result.current.items.find((candidate) => candidate.iconKey === key);
      if (item?.type !== 'action') throw new Error(`${key} must be a card action`);
      return item.action;
    };

    act(() => actionFor('search')());
    expect(result.current.showSearch).toBe(true);
    expect(useOcrScanStore.getState().isCardOpen).toBe(false);
    expect(usePluginQuickStore.getState().isOpen).toBe(false);

    act(() => actionFor('ocr')());
    expect(useOcrScanStore.getState().isCardOpen).toBe(true);
    expect(usePluginQuickStore.getState().isOpen).toBe(false);

    act(() => actionFor('plugins')());
    expect(usePluginQuickStore.getState().isOpen).toBe(true);
    expect(result.current.showSearch).toBe(true);

    act(() => actionFor('search')());
    expect(result.current.showSearch).toBe(false);
  });

  it('desktop page mode routes each configurable action to its dedicated page', () => {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        sidebarButtonModes: { search: 'page', ocr: 'page', plugins: 'page', ai_chat: 'page' },
      },
    });
    const { result } = renderHook(() => useBoundNavActions());
    const paths = new Map(
      result.current.items
        .filter((item) => item.type === 'link')
        .map((item) => [item.iconKey, item.path]),
    );

    expect(paths.get('search')).toBe('/search');
    expect(paths.get('ocr')).toBe('/ocr');
    expect(paths.get('plugins')).toBe('/plugins');
    expect(paths.get('ai_chat')).toBe('/llm-chat');
    expect(paths.get('import_export')).toBe('/settings/export-import');
  });

  it('mobile navigation uses pages even when desktop buttons use cards', () => {
    const { result } = renderHook(() => useMobileNavActions());
    const paths = new Map(
      result.current.items
        .filter((item) => item.type === 'link')
        .map((item) => [item.iconKey, item.path]),
    );

    expect(paths.get('search')).toBe('/search');
    expect(paths.get('ocr')).toBe('/ocr');
    expect(paths.get('plugins')).toBe('/plugins');
    expect(paths.get('ai_chat')).toBe('/llm-chat');
    expect(result.current.items.every((item) => item.type === 'link')).toBe(true);
  });
});
