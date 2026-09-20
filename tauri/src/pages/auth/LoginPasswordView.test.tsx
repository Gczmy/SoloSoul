import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LoginPasswordView } from './LoginPasswordView';

describe('LoginPasswordView', () => {
  const baseProps = {
    password: '',
    onPasswordChange: vi.fn(),
    isLoading: false,
    bioError: null,
    submitError: null,
    pinError: null,
    passwordFieldError: null,
    passwordErrorTick: 0,
    passwordHint: null,
    onSubmit: vi.fn(),
  };

  it('renders passwordFieldError inline inside the password input, not duplicated in the standalone error area', () => {
    render(<LoginPasswordView {...baseProps} passwordFieldError="common:invalid_password" />);
    // 主密码错误仍由输入框展示，不增加第二个重复提示。
    expect(screen.getAllByRole('alert')).toHaveLength(1);
    expect(screen.getByRole('alert')).toHaveTextContent('common:invalid_password');
  });

  it('announces a submit error when present', () => {
    render(<LoginPasswordView {...baseProps} submitError="auth:no_account_selected" />);
    expect(screen.getByRole('alert')).toHaveTextContent('auth:no_account_selected');
  });

  it('renders both independently when both password and submit errors exist', () => {
    render(
      <LoginPasswordView
        {...baseProps}
        passwordFieldError="common:invalid_password"
        submitError="auth:no_account_selected"
      />,
    );
    expect(screen.getAllByRole('alert').map((alert) => alert.textContent)).toEqual([
      'common:invalid_password',
      'auth:no_account_selected',
    ]);
  });

  it('removes the standalone alert after the error is cleared', () => {
    const { rerender } = render(<LoginPasswordView {...baseProps} />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();

    rerender(<LoginPasswordView {...baseProps} submitError="auth:no_account_selected" />);
    expect(screen.getByRole('alert')).toHaveTextContent('auth:no_account_selected');

    rerender(<LoginPasswordView {...baseProps} />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'auth:login_button' })).toBeEnabled();
  });

  it('keeps PIN and submit errors ahead of a biometric fallback error', () => {
    const { rerender } = render(
      <LoginPasswordView
        {...baseProps}
        pinError="PIN error"
        submitError="Submit error"
        bioError="Biometric error"
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent('PIN error');
    expect(screen.queryByText('Submit error')).not.toBeInTheDocument();
    expect(screen.queryByText('Biometric error')).not.toBeInTheDocument();

    rerender(
      <LoginPasswordView {...baseProps} submitError="Submit error" bioError="Biometric error" />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Submit error');
  });
});
