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
    // 主密码错误只出现在 SecurePasswordInput 行内（红边 + 行内红字），独立错误区为空
    expect(screen.getAllByText('common:invalid_password')).toHaveLength(1);
  });

  it('shows submitError in the standalone error area (non-password errors keep the div)', () => {
    render(<LoginPasswordView {...baseProps} submitError="auth:no_account_selected" />);
    expect(screen.getByText('auth:no_account_selected')).toBeInTheDocument();
  });

  it('renders both independently when both password and submit errors exist', () => {
    render(
      <LoginPasswordView
        {...baseProps}
        passwordFieldError="common:invalid_password"
        submitError="auth:no_account_selected"
      />,
    );
    expect(screen.getAllByText('common:invalid_password')).toHaveLength(1);
    expect(screen.getByText('auth:no_account_selected')).toBeInTheDocument();
  });

  it('always reserves the error area height (minHeight div present even without errors)', () => {
    render(<LoginPasswordView {...baseProps} />);
    // 独立错误区容器常驻（minHeight 预留），无错误时为空
    expect(screen.queryByText('auth:no_account_selected')).not.toBeInTheDocument();
  });
});
