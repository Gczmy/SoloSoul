import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LoginPage } from './LoginPage';
import type { LoginMethodOption } from './LoginIconBar';

// 组合层纯展示测试：mock useLoginPage hook 控制 loginMethod，
// 验证「图标栏常驻 + 探测中显示占位而非主密码」的防闪烁渲染结构。
const { mockUseLoginPage } = vi.hoisted(() => ({ mockUseLoginPage: vi.fn() }));
vi.mock('./useLoginPage', () => ({
  useLoginPage: () => mockUseLoginPage(),
}));

function baseHook() {
  const passwordIcon: LoginMethodOption = {
    id: 'password',
    icon: <span />,
    label: '主密码',
    onClick: vi.fn(),
  };
  return {
    accounts: [{ id: 'acc-1', name: 'Gczmy' }],
    selectedAccountId: 'acc-1',
    setSelectedAccountId: vi.fn(),
    selectedAccount: { id: 'acc-1', name: 'Gczmy' },
    password: '',
    setPassword: vi.fn(),
    passwordFieldError: null,
    setPasswordFieldError: vi.fn(),
    passwordErrorTick: 0,
    isLoading: false,
    loginMethod: null as 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password' | null,
    bioLoading: false,
    bioLockout: false,
    bioError: null,
    pinUnlocking: false,
    pinError: null,
    pinInputKey: 0,
    pinInputRef: { current: null },
    submitError: null,
    handleBiometricUnlock: vi.fn(),
    handlePinComplete: vi.fn(),
    handleSubmit: vi.fn(),
    iconMethods: [passwordIcon],
    hoveredIcon: null,
    committedIcon: null,
    handleIconEnter: vi.fn(),
    handleIconLeave: vi.fn(),
    handleIconClick: vi.fn(),
    recoveryOpen: false,
    setRecoveryOpen: vi.fn(),
    listAccounts: vi.fn(),
    navigate: vi.fn(),
    t1FiredRef: { current: false },
  };
}

describe('LoginPage 登录方式渲染（防闪烁，方案 B）', () => {
  beforeEach(() => {
    mockUseLoginPage.mockReturnValue(baseHook());
  });

  it('loginMethod 为 null（可用性探测中）：不渲染主密码表单，渲染固定高度占位', () => {
    render(<LoginPage />);
    // 占位（minHeight 152 与各视图一致）——不再「先闪主密码」
    expect(screen.getByTestId('login-method-placeholder')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'auth:login_button' })).not.toBeInTheDocument();
  });

  it('图标栏常驻：loginMethod 为 null 时仍渲染（不再绑定 loginMethod !== null）', () => {
    render(<LoginPage />);
    expect(screen.getByRole('button', { name: '主密码' })).toBeInTheDocument();
  });

  it('loginMethod 为 password：渲染主密码表单，占位消失', () => {
    mockUseLoginPage.mockReturnValue({ ...baseHook(), loginMethod: 'password' });
    render(<LoginPage />);
    expect(screen.queryByTestId('login-method-placeholder')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'auth:login_button' })).toBeInTheDocument();
  });

  it('loginMethod 为 touchId：渲染指纹视图，不渲染主密码表单', () => {
    mockUseLoginPage.mockReturnValue({ ...baseHook(), loginMethod: 'touchId' });
    render(<LoginPage />);
    expect(screen.getByText('auth:bio_unlock_reason')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'auth:login_button' })).not.toBeInTheDocument();
  });
});
