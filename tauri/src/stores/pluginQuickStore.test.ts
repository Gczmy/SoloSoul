import { describe, it, expect, beforeEach } from 'vitest';
import { usePluginQuickStore } from './pluginQuickStore';

describe('pluginQuickStore', () => {
  // Zustand stores persist state across tests; reset before each.
  beforeEach(() => {
    usePluginQuickStore.setState({ isOpen: false, activeTab: 'all' });
  });

  it('starts with isOpen=false and activeTab=all', () => {
    const state = usePluginQuickStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.activeTab).toBe('all');
  });

  describe('setOpen', () => {
    it('opens the panel when called with true', () => {
      usePluginQuickStore.getState().setOpen(true);
      expect(usePluginQuickStore.getState().isOpen).toBe(true);
    });

    it('closes the panel when called with false', () => {
      usePluginQuickStore.getState().setOpen(true);
      usePluginQuickStore.getState().setOpen(false);
      expect(usePluginQuickStore.getState().isOpen).toBe(false);
    });
  });

  describe('toggleOpen', () => {
    it('toggles from closed to open', () => {
      usePluginQuickStore.getState().toggleOpen();
      expect(usePluginQuickStore.getState().isOpen).toBe(true);
    });

    it('toggles from open to closed', () => {
      usePluginQuickStore.getState().setOpen(true);
      usePluginQuickStore.getState().toggleOpen();
      expect(usePluginQuickStore.getState().isOpen).toBe(false);
    });
  });

  describe('setActiveTab', () => {
    it('sets tab to installed', () => {
      usePluginQuickStore.getState().setActiveTab('installed');
      expect(usePluginQuickStore.getState().activeTab).toBe('installed');
    });

    it('sets tab to running', () => {
      usePluginQuickStore.getState().setActiveTab('running');
      expect(usePluginQuickStore.getState().activeTab).toBe('running');
    });

    it('sets tab back to all', () => {
      usePluginQuickStore.getState().setActiveTab('running');
      usePluginQuickStore.getState().setActiveTab('all');
      expect(usePluginQuickStore.getState().activeTab).toBe('all');
    });
  });
});
