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
      </MemoryRouter>,
    );

    expect(screen.getByText('auth:bootstrap_title')).toBeInTheDocument();
    expect(screen.getByText('auth:bootstrap_subtitle')).toBeInTheDocument();
    expect(screen.getAllByPlaceholderText('common:password_placeholder')).toHaveLength(2);
  });

  it('updates input values on change', () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>,
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
      </MemoryRouter>,
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    const passwordInputs = screen.getAllByPlaceholderText('common:password_placeholder');

    fireEvent.change(accountInput, { target: { value: 'Alice' } });
    fireEvent.change(passwordInputs[0], { target: { value: 'password123' } });
    fireEvent.change(passwordInputs[1], { target: { value: 'password123' } });

    const form = accountInput.closest('form') as HTMLFormElement;
    fireEvent.submit(form);

    await waitFor(() => {
      expect(bootstrapMock).toHaveBeenCalledWith(
        'Alice',
        'password123',
        expect.any(String),
        undefined,
      );
    });
    expect(navigate).toHaveBeenCalledWith('/');
  });

  it('shows account name error first when submitting empty form (priority 1)', async () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>,
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    const form = accountInput.closest('form') as HTMLFormElement;
    fireEvent.submit(form);

    expect(await screen.findByText('auth:account_name_required')).toBeInTheDocument();
    expect(screen.queryByText('auth:master_password_required')).not.toBeInTheDocument();
    expect(bootstrapMock).not.toHaveBeenCalled();
  });

  it('shows master password error when only account name is filled (priority 2)', async () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>,
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    fireEvent.change(accountInput, { target: { value: 'Alice' } });
    const form = accountInput.closest('form') as HTMLFormElement;
    fireEvent.submit(form);

    expect(await screen.findByText('auth:master_password_required')).toBeInTheDocument();
    expect(screen.queryByText('auth:account_name_required')).not.toBeInTheDocument();
    expect(screen.queryByText('auth:confirm_password_required')).not.toBeInTheDocument();
    expect(bootstrapMock).not.toHaveBeenCalled();
  });

  it('shows confirm password error when passwords are empty (priority 3)', async () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>,
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    const passwordInputs = screen.getAllByPlaceholderText('common:password_placeholder');
    fireEvent.change(accountInput, { target: { value: 'Alice' } });
    fireEvent.change(passwordInputs[0], { target: { value: 'password123' } });
    // 确认密码留空
    const form = accountInput.closest('form') as HTMLFormElement;
    fireEvent.submit(form);

    expect(await screen.findByText('auth:confirm_password_required')).toBeInTheDocument();
    expect(screen.queryByText('auth:master_password_required')).not.toBeInTheDocument();
    expect(bootstrapMock).not.toHaveBeenCalled();
  });

  it('clears field errors while typing', async () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>,
    );

    const accountInput = screen.getByPlaceholderText('auth:account_name');
    const form = accountInput.closest('form') as HTMLFormElement;
    fireEvent.submit(form);
    expect(await screen.findByText('auth:account_name_required')).toBeInTheDocument();

    fireEvent.change(accountInput, { target: { value: 'Alice' } });
    expect(screen.queryByText('auth:account_name_required')).not.toBeInTheDocument();
  });

  it('does not call bootstrap when passwords do not match', async () => {
    render(
      <MemoryRouter>
        <BootstrapPage />
      </MemoryRouter>,
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
      </MemoryRouter>,
    );

    expect(screen.getByText('some backend error')).toBeInTheDocument();
  });

  it('shows back-to-login link when opened with mode=create', () => {
    render(
      <MemoryRouter initialEntries={['/bootstrap?mode=create']}>
        <BootstrapPage />
      </MemoryRouter>,
    );

    const backLink = screen.getByText('common:back_to_login_link');
    expect(backLink).toBeInTheDocument();
    fireEvent.click(backLink);
    expect(navigate).toHaveBeenCalledWith('/login');
  });

  it('does not show back-to-login link without mode=create', () => {
    render(
      <MemoryRouter initialEntries={['/bootstrap']}>
        <BootstrapPage />
      </MemoryRouter>,
    );

    expect(screen.queryByText('common:back_to_login_link')).not.toBeInTheDocument();
  });
});
