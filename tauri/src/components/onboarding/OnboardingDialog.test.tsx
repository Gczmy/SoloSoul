import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { OnboardingDialog } from './OnboardingDialog';
import * as vaultDirectory from '@/lib/vaultDirectory';
import { getPlatform } from '@/lib/platform';
import { ST_ONBOARDING_SAF_URI } from '@/lib/constants';

const renderWithRouter = (ui: React.ReactElement) => render(<BrowserRouter>{ui}</BrowserRouter>);

vi.mock('@/lib/platform', () => ({
  getPlatform: vi.fn().mockResolvedValue('web'),
  isMobilePlatform: vi.fn().mockResolvedValue(false),
  isMobilePlatformSync: vi.fn().mockReturnValue(false),
  initPlatform: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/lib/vaultDirectory', () => ({
  pickVaultDirectory: vi.fn(),
  initVaultDirectory: vi.fn(),
}));

const mockCheckHasAccount = vi.fn().mockResolvedValue(undefined);

vi.mock('@/stores/authStore', () => ({
  useAuthStore: Object.assign(
    vi.fn(() => ({
      hasAccount: false,
      checkHasAccount: mockCheckHasAccount,
    })),
    {
      getState: () => ({
        hasAccount: false,
        checkHasAccount: mockCheckHasAccount,
      }),
      setState: vi.fn(),
    },
  ),
}));

describe('OnboardingDialog', () => {
  it('renders the first step by default', async () => {
    renderWithRouter(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/onboarding_welcome_desc/i)).toBeInTheDocument();
  });

  it('advances to the next step when clicking next', async () => {
    renderWithRouter(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    // Wait for platform mock to resolve (vault_directory filtered out on 'web')
    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_create_object_title/i)).toBeInTheDocument();
    });
  });

  it('shows the recovery receive dialog when choosing sync from another device', async () => {
    const onComplete = vi.fn();
    renderWithRouter(<OnboardingDialog onComplete={onComplete} onSkip={vi.fn()} />);

    // Wait for platform to load
    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });

    // Advance through all steps
    const nextButton = screen.getByRole('button', { name: /onboarding_next/i });
    fireEvent.click(nextButton);
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    expect(screen.queryByRole('button', { name: /onboarding_next/i })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /onboarding_done/i }));

    // Decision card appears because no existing account is found
    await waitFor(() => {
      expect(screen.getByText(/onboarding_account_source_title/i)).toBeInTheDocument();
    });

    // Click sync from another device to open the recovery dialog
    fireEvent.click(screen.getByText(/onboarding_account_source_sync/i));
    await waitFor(() => {
      expect(screen.getByText(/recovery_receive_title/i)).toBeInTheDocument();
    });
  });

  it('shows the account source decision when no existing account is found', async () => {
    const onComplete = vi.fn();
    renderWithRouter(<OnboardingDialog onComplete={onComplete} onSkip={vi.fn()} />);

    // Wait for platform to load
    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });

    // Advance through all steps
    const nextButton = screen.getByRole('button', { name: /onboarding_next/i });
    fireEvent.click(nextButton);
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    expect(screen.queryByRole('button', { name: /onboarding_next/i })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /onboarding_done/i }));

    // Decision card appears because no existing account is found
    await waitFor(() => {
      expect(screen.getByText(/onboarding_account_source_title/i)).toBeInTheDocument();
    });

    // Click create new account to finish onboarding
    fireEvent.click(screen.getByText(/onboarding_account_source_create/i));
    await waitFor(() => {
      expect(onComplete).toHaveBeenCalledTimes(1);
    });
  });

  it('goes back to the previous step when clicking back', async () => {
    renderWithRouter(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    // Wait for platform to load
    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    await waitFor(() => {
      expect(screen.getByText(/onboarding_create_object_title/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /onboarding_back/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
  });
});

describe('OnboardingDialog vault directory step (Android)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    localStorage.removeItem(ST_ONBOARDING_SAF_URI);
  });

  it('shows selected external path and next button after picking SAF directory', async () => {
    vi.mocked(getPlatform).mockResolvedValue('android');
    vi.mocked(vaultDirectory.pickVaultDirectory).mockResolvedValue(
      'content://com.android.documents/tree/primary%3ADocuments%2FSoloSoul',
    );
    vi.mocked(vaultDirectory.initVaultDirectory).mockResolvedValue({
      success: true,
      needsRestart: false,
      message: '',
    });

    renderWithRouter(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    // Android still starts from the welcome step; navigate to vault directory step
    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_title/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/onboarding_vault_dir_saf_title/i));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_selected_label/i)).toBeInTheDocument();
    }, { timeout: 4000 });
    expect(
      screen.getByText(
        /content:\/\/com.android.documents\/tree\/primary%3ADocuments%2FSoloSoul/i,
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /onboarding_next/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_create_object_title/i)).toBeInTheDocument();
    });
  });

  it('preserves selected external path when going back from the next step', async () => {
    vi.mocked(getPlatform).mockResolvedValue('android');
    vi.mocked(vaultDirectory.pickVaultDirectory).mockResolvedValue(
      'content://com.android.documents/tree/primary%3ADocuments%2FSoloSoul',
    );
    vi.mocked(vaultDirectory.initVaultDirectory).mockResolvedValue({
      success: true,
      needsRestart: false,
      message: '',
    });

    renderWithRouter(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText(/onboarding_vault_dir_saf_title/i));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_selected_label/i)).toBeInTheDocument();
    }, { timeout: 4000 });

    // Advance to next step, then go back
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    await waitFor(() => {
      expect(screen.getByText(/onboarding_create_object_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /onboarding_back/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_title/i)).toBeInTheDocument();
      expect(
        screen.getByText(
          /content:\/\/com.android.documents\/tree\/primary%3ADocuments%2FSoloSoul/i,
        ),
      ).toBeInTheDocument();
    });
  });

  it('preserves selected external path when going back to welcome and forward again', async () => {
    vi.mocked(getPlatform).mockResolvedValue('android');
    vi.mocked(vaultDirectory.pickVaultDirectory).mockResolvedValue(
      'content://com.android.documents/tree/primary%3ADocuments%2FSoloSoul',
    );
    vi.mocked(vaultDirectory.initVaultDirectory).mockResolvedValue({
      success: true,
      needsRestart: false,
      message: '',
    });

    renderWithRouter(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText(/onboarding_vault_dir_saf_title/i));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_selected_label/i)).toBeInTheDocument();
    }, { timeout: 4000 });

    // Go back to welcome, then forward to vault directory again
    fireEvent.click(screen.getByRole('button', { name: /onboarding_back/i }));
    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_title/i)).toBeInTheDocument();
      expect(
        screen.getByText(
          /content:\/\/com.android.documents\/tree\/primary%3ADocuments%2FSoloSoul/i,
        ),
      ).toBeInTheDocument();
    });
  });

  it('restores selected external path from localStorage after component remount', async () => {
    vi.mocked(getPlatform).mockResolvedValue('android');
    vi.mocked(vaultDirectory.initVaultDirectory).mockResolvedValue({
      success: true,
      needsRestart: false,
      message: '',
    });

    // Simulate a previously selected SAF URI that survived an activity rebuild
    localStorage.setItem(
      ST_ONBOARDING_SAF_URI,
      'content://com.android.documents/tree/primary%3ADocuments%2FSoloSoul',
    );

    const { unmount } = renderWithRouter(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    // Because the cached SAF URI is restored, the vault directory step should
    // show the selected path summary instead of the picker buttons.
    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_title/i)).toBeInTheDocument();
      expect(
        screen.getByText(
          /content:\/\/com.android.documents\/tree\/primary%3ADocuments%2FSoloSoul/i,
        ),
      ).toBeInTheDocument();
    });

    // Persistence contract: a restored SAF URI should be used as-is without
    // re-prompting the user to pick a directory.
    expect(vaultDirectory.pickVaultDirectory).not.toHaveBeenCalled();

    // Unmount and remount to simulate activity destruction/recreation
    unmount();
    const { unmount: unmount2 } = renderWithRouter(
      <OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_vault_dir_title/i)).toBeInTheDocument();
      expect(
        screen.getByText(
          /content:\/\/com.android.documents\/tree\/primary%3ADocuments%2FSoloSoul/i,
        ),
      ).toBeInTheDocument();
    });

    unmount2();
  });
});
