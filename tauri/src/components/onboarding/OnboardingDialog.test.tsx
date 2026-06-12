import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OnboardingDialog } from './OnboardingDialog';

describe('OnboardingDialog', () => {
  it('renders the first step by default', () => {
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
    expect(screen.getByText(/onboarding_welcome_desc/i)).toBeInTheDocument();
  });

  it('advances to the next step when clicking next', () => {
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    expect(screen.getByText(/onboarding_create_object_title/i)).toBeInTheDocument();
  });

  it('shows the done button on the last step and calls onComplete', () => {
    const onComplete = vi.fn();
    render(<OnboardingDialog onComplete={onComplete} onSkip={vi.fn()} />);

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

  it('calls onSkip when clicking skip', () => {
    const onSkip = vi.fn();
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={onSkip} />);

    fireEvent.click(screen.getByRole('button', { name: /onboarding_skip/i }));
    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  it('goes back to the previous step when clicking back', () => {
    render(<OnboardingDialog onComplete={vi.fn()} onSkip={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /onboarding_next/i }));
    expect(screen.getByText(/onboarding_create_object_title/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /onboarding_back/i }));
    expect(screen.getByText(/onboarding_welcome_title/i)).toBeInTheDocument();
  });
});
