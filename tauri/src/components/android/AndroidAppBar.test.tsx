import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';
import { AndroidAppBar } from './AndroidAppBar';

const originalLock = useAuthStore.getState().lock;

function CurrentPath() {
  const { pathname } = useLocation();
  return <output data-testid="current-path">{pathname}</output>;
}

function showAppBar(path: string, props: Partial<Parameters<typeof AndroidAppBar>[0]> = {}) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <AndroidAppBar title="Current page" {...props} />
      <CurrentPath />
    </MemoryRouter>,
  );
}

afterEach(() => {
  act(() => useAuthStore.setState({ lock: originalLock }));
});

describe('AndroidAppBar actions', () => {
  it('locks the vault and opens account settings from the home bar', () => {
    const lock = vi.fn().mockResolvedValue(undefined);
    act(() => useAuthStore.setState({ lock }));
    showAppBar('/');

    expect(screen.getByRole('heading', { name: 'SoloSoul' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'lock_vault' }));
    expect(lock).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: 'common:material.account' }));
    expect(screen.getByTestId('current-path')).toHaveTextContent('/settings/account');
  });

  it('uses the provided back action on a nested page', () => {
    const onBack = vi.fn();
    showAppBar('/settings/template', { title: 'Templates', onBack });

    expect(screen.getByRole('heading', { name: 'Templates' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'common:back' }));
    expect(onBack).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('button', { name: 'common:material.account' })).toBeNull();
  });

  it('does not show a nested back button on a root destination', () => {
    showAppBar('/settings', { title: 'Settings', onBack: vi.fn() });
    expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'common:back' })).toBeNull();
  });
});
