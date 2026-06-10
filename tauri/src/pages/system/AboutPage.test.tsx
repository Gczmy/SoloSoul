import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AboutPage } from './AboutPage';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@/components/layout/AppShell', () => ({
  AppShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="app-shell" data-title={title}>{children}</div>
  ),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('AboutPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state initially', () => {
    vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));
    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>
    );
    expect(screen.getByText('settings:loading')).toBeInTheDocument();
  });

  it('renders app info after loading', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_app_info') {
        return { appName: 'SoloSoul', version: '1.2.3', os: 'macos', arch: 'aarch64' };
      }
      if (cmd === 'check_version') {
        return { currentVersion: '1.2.3', latestVersion: '1.2.3', hasUpdate: false };
      }
      return undefined;
    });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('v1.2.3')).toBeInTheDocument();
    });

    expect(screen.getByText('SoloSoul')).toBeInTheDocument();
    expect(screen.getByText('macOS (Apple Silicon)')).toBeInTheDocument();
    expect(screen.getByText('settings:latest_version')).toBeInTheDocument();
  });

  it('shows update available badge when new version exists', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_app_info') {
        return { appName: 'SoloSoul', version: '1.0.0', os: 'windows', arch: 'x86_64' };
      }
      if (cmd === 'check_version') {
        return { currentVersion: '1.0.0', latestVersion: '1.2.0', hasUpdate: true };
      }
      return undefined;
    });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText(/settings:update_available/)).toBeInTheDocument();
    });

    expect(screen.getByText('Windows (Intel)')).toBeInTheDocument();
  });

  it('renders external links', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_app_info') {
        return { appName: 'SoloSoul', version: '1.0.0', os: 'linux', arch: 'x86_64' };
      }
      if (cmd === 'check_version') {
        return { currentVersion: '1.0.0', latestVersion: null, hasUpdate: false };
      }
      return undefined;
    });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('settings:github_repo')).toBeInTheDocument();
    });

    expect(screen.getByText('settings:privacy_policy')).toBeInTheDocument();
    expect(screen.getByText('settings:terms_of_service')).toBeInTheDocument();
  });

  it('handles fetch errors gracefully', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('backend offline'));

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('settings:could_not_load')).toBeInTheDocument();
    });
  });
});
