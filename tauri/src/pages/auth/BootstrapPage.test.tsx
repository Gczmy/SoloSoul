import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import { BootstrapPage } from './BootstrapPage';
import { useAuthStore } from '@/stores/authStore';

const bootstrapMock = vi.fn();

vi.mock('@/stores/authStore', () => ({
  useAuthStore: vi.fn(() => ({
    bootstrap: bootstrapMock,
    isLoading: false,
    error: null,
  })),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: vi.fn(),
  };
});

describe('BootstrapPage', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(navigate);
    vi.mocked(useAuthStore).mockReturnValue({
      bootstrap: bootstrapMock,
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useAuthStore>);
  });

  it('renders bootstrap form', () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>
    );

    expect(screen.getByText('auth:bootstrap_title')).toBeInTheDocument();
    expect(screen.getByText('auth:bootstrap_subtitle')).toBeInTheDocument();
    expect(screen.getAllByPlaceholderText('common:password_placeholder')).toHaveLength(2);
  });

  it('updates input values on change', () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    fireEvent.change(accountInput, { target: { value: 'Alice' } });
    expect(accountInput).toHaveValue('Alice');
  });

  it('calls bootstrap and navigates on submit', async () => {
    bootstrapMock.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    const passwordInputs = screen.getAllByPlaceholderText('common:password_placeholder');

    fireEvent.change(accountInput, { target: { value: 'Alice' } });
    fireEvent.change(passwordInputs[0], { target: { value: 'password123' } });
    fireEvent.change(passwordInputs[1], { target: { value: 'password123' } });

    const form = accountInput.closest('form') as HTMLFormElement;
    fireEvent.submit(form);

    await waitFor(() => {
      expect(bootstrapMock).toHaveBeenCalledWith('Alice', 'password123');
    });
    expect(navigate).toHaveBeenCalledWith('/');
  });

  it('does not call bootstrap when passwords do not match', async () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    const passwordInputs = screen.getAllByPlaceholderText('common:password_placeholder');

    fireEvent.change(accountInput, { target: { value: 'Alice' } });
    fireEvent.change(passwordInputs[0], { target: { value: 'password123' } });
    fireEvent.change(passwordInputs[1], { target: { value: 'different' } });

    const form = accountInput.closest('form') as HTMLFormElement;
    fireEvent.submit(form);

    await waitFor(() => {
      expect(bootstrapMock).not.toHaveBeenCalled();
    });
  });

  it('displays error from authStore', () => {
    vi.mocked(useAuthStore).mockReturnValue({
      bootstrap: bootstrapMock,
      isLoading: false,
      error: 'some backend error',
    } as unknown as ReturnType<typeof useAuthStore>);

    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>
    );

    expect(screen.getByText('some backend error')).toBeInTheDocument();
  });
});
