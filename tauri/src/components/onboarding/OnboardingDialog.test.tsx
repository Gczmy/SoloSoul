import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { OnboardingDialog } from './OnboardingDialog';

vi.mock('@/lib/platform', () => ({
  getPlatform: vi.fn().mockResolvedValue('web'),
  isMobilePlatform: vi.fn().mockResolvedValue(false),
  isMobilePlatformSync: vi.fn().mockReturnValue(false),
  initPlatform: vi.fn().mockResolvedValue(undefined),
}));

describe('OnboardingDialog', () => {
  it('renders the first step by default', async () => {
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/onboarding_welcome_desc/i)).toBeInTheDocument();
  });

  it('advances to the next step when clicking next', async () => {
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    // Wait for platform mock to resolve (vault_directory filtered out on 'web')
    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));

    await waitFor(() => {
      expect(screen.getByText(/onboarding_create_object_title/i)).toBeInTheDocument();
    });
  });

  it('shows the done button on the last step and calls onComplete', async () => {
    const onComplete = vi.fn();
    render(<OnboardingDialog onComplete={onComplete} onSkip={vi.fn()} />);

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
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it('calls onSkip when clicking skip', async () => {
    const onSkip = vi.fn();
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={onSkip} />);

    await waitFor(() => {
      expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /onboarding_skip/i }));
    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  it('goes back to the previous step when clicking back', async () => {
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

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
