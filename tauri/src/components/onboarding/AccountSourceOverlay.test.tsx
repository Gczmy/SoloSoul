import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { AccountSourceOverlay } from './AccountSourceOverlay';
import { useUiStore } from '@/stores/uiStore';

const mockNavigate = vi.fn();

vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

// 恢复对话框的编排（步骤机/扫码）由 RecoveryReceiveDialog 自身测试覆盖，
// 此处 mock 以聚焦浮层的互斥显示与标志清理逻辑。
vi.mock('@/components/recovery/RecoveryReceiveDialog', () => ({
  RecoveryReceiveDialog: ({
    isOpen,
    onClose,
    onSuccess,
  }: {
    isOpen: boolean;
    onClose: () => void;
    onSuccess?: () => void;
  }) =>
    isOpen ? (
      <div data-testid="recovery-dialog">
        <button onClick={onClose}>close-recovery</button>
        <button onClick={onSuccess}>success-recovery</button>
      </div>
    ) : null,
}));

const renderOverlay = () =>
  render(
    <BrowserRouter>
      <AccountSourceOverlay />
    </BrowserRouter>,
  );

describe('AccountSourceOverlay', () => {
  beforeEach(() => {
    useUiStore.getState().setReopenAccountSource(true);
    mockNavigate.mockClear();
  });

  afterEach(() => {
    useUiStore.getState().setReopenAccountSource(false);
  });

  it('renders the decision card immediately without the onboarding wizard underneath', () => {
    renderOverlay();

    expect(screen.getByText(/onboarding_account_source_title/i)).toBeInTheDocument();
    expect(screen.getByText(/onboarding_account_source_create/i)).toBeInTheDocument();
    // 重开场景不再挂载整个引导向导：不出现向导步骤卡片
    expect(screen.queryByText(/onboarding_welcome_title/i)).not.toBeInTheDocument();
  });

  it('clears reopenAccountSource when clicking back (reveals the bootstrap form)', () => {
    renderOverlay();

    fireEvent.click(screen.getByText(/onboarding_account_source_back/i));

    expect(useUiStore.getState().reopenAccountSource).toBe(false);
    // 「返回」只关浮层，不做任何导航
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('clears the flag and stays on the create-account page when creating a new account', () => {
    renderOverlay();

    fireEvent.click(screen.getByText(/onboarding_account_source_create/i));

    expect(useUiStore.getState().reopenAccountSource).toBe(false);
    expect(mockNavigate).toHaveBeenCalledWith('/bootstrap?mode=create', { replace: true });
  });

  it('shows the recovery dialog after choosing recovery, and returns to the decision card on close', () => {
    renderOverlay();

    fireEvent.click(screen.getByText(/onboarding_account_source_sync/i));

    // 决策卡片隐藏、恢复对话框显示（互斥）
    expect(screen.queryByText(/onboarding_account_source_title/i)).not.toBeInTheDocument();
    expect(screen.getByTestId('recovery-dialog')).toBeInTheDocument();

    // 关闭恢复对话框 → 回到决策卡片
    fireEvent.click(screen.getByText('close-recovery'));
    expect(screen.getByText(/onboarding_account_source_title/i)).toBeInTheDocument();
    expect(screen.queryByTestId('recovery-dialog')).not.toBeInTheDocument();
  });

  it('clears the flag and navigates to login after recovery succeeds', () => {
    renderOverlay();

    fireEvent.click(screen.getByText(/onboarding_account_source_sync/i));
    fireEvent.click(screen.getByText('success-recovery'));

    expect(useUiStore.getState().reopenAccountSource).toBe(false);
    expect(mockNavigate).toHaveBeenCalledWith('/login', { replace: true });
  });
});
