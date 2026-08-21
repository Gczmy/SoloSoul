import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ObjectDetailFieldsList, type FlattenedField } from './ObjectDetailFieldsList';
import { useRevealState } from '@/hooks/useRevealState';
import type { SensitivityLevel, TemplateProperty } from '@/types/template';

// 使用真实 useRevealState（含真实 maskValue 逻辑），验证详情卡片掩码规则：
// - internal / public：直接显示明文（无揭示按钮）；
// - sensitive / critical：掩码 + 揭示按钮（critical 弹密码）。
vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

vi.mock('@/lib/platform', () => ({
  isMobilePlatformSync: vi.fn(() => false),
}));

function Harness({
  fields,
  sensitivities,
}: {
  fields: FlattenedField[];
  sensitivities: Record<string, SensitivityLevel>;
}) {
  const revealState = useRevealState();
  return (
    <ObjectDetailFieldsList
      fields={fields}
      typeId="travel"
      contractTypeId={undefined}
      objFieldDefs={undefined}
      getFieldProperty={(k) =>
        ({ id: k, sensitivityLevel: sensitivities[k] }) as unknown as TemplateProperty
      }
      getFieldSensitivity={(k) => sensitivities[k] || 'internal'}
      isFieldDeprecated={() => false}
      getFieldName={(k, label) => label ?? k}
      isRevealed={revealState.isRevealed}
      revealRemainingMs={revealState.revealRemainingMs}
      maskValue={revealState.maskValue}
      handleRevealField={vi.fn()}
      handleCopy={vi.fn()}
      copiedField={null}
    />
  );
}

describe('ObjectDetailFieldsList 掩码规则', () => {
  it('internal 字段：直接显示明文，无揭示按钮', () => {
    render(
      <Harness
        fields={[{ key: 'phone', value: '13800138000' }]}
        sensitivities={{ phone: 'internal' }}
      />,
    );
    // 明文直接可见
    expect(screen.getByText('13800138000')).toBeInTheDocument();
    // 无掩码圆点、无揭示按钮
    expect(screen.queryByText('••••••••')).not.toBeInTheDocument();
    expect(screen.queryByText('common:reveal')).not.toBeInTheDocument();
  });

  it('public 字段：直接显示明文，无揭示按钮', () => {
    render(
      <Harness
        fields={[{ key: 'nickname', value: 'Alice' }]}
        sensitivities={{ nickname: 'public' }}
      />,
    );
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.queryByText('••••••••')).not.toBeInTheDocument();
  });

  it('sensitive 字段：掩码为 8 圆点 + 显示揭示按钮', () => {
    render(
      <Harness
        fields={[{ key: 'email', value: 'secret@example.com' }]}
        sensitivities={{ email: 'sensitive' }}
      />,
    );
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    expect(screen.queryByText('secret@example.com')).not.toBeInTheDocument();
    expect(screen.getByText('common:reveal')).toBeInTheDocument();
  });

  it('critical 字段：掩码 + 显示解锁按钮', () => {
    render(
      <Harness
        fields={[{ key: 'password', value: 'p@ss' }]}
        sensitivities={{ password: 'critical' }}
      />,
    );
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    expect(screen.queryByText('p@ss')).not.toBeInTheDocument();
    expect(screen.getByText('common:unlock')).toBeInTheDocument();
  });

  it('sensitive 字段点击揭示后显示明文（reveal 链路完整）', () => {
    // 模拟 modal 层 handleRevealField 的真实行为：非 critical 直接 reveal
    let capturedId: string | null = null;
    function HarnessWithReveal() {
      const revealState = useRevealState();
      return (
        <ObjectDetailFieldsList
          fields={[{ key: 'email', value: 'secret@example.com' }]}
          typeId="travel"
          contractTypeId={undefined}
          objFieldDefs={undefined}
          getFieldProperty={(k) =>
            ({ id: k, sensitivityLevel: 'sensitive' }) as unknown as TemplateProperty
          }
          getFieldSensitivity={() => 'sensitive'}
          isFieldDeprecated={() => false}
          getFieldName={(k, label) => label ?? k}
          isRevealed={revealState.isRevealed}
          revealRemainingMs={revealState.revealRemainingMs}
          maskValue={revealState.maskValue}
          handleRevealField={(id) => {
            capturedId = id;
            revealState.reveal(id);
          }}
          handleCopy={vi.fn()}
          copiedField={null}
        />
      );
    }
    render(<HarnessWithReveal />);
    // 初始掩码
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    // 点击揭示 → 掩码占位消失、明文出现
    fireEvent.click(screen.getByText('common:reveal'));
    expect(capturedId).toBe('travel.email');
    expect(screen.queryByText('••••••••')).not.toBeInTheDocument();
    expect(screen.getByText('secret@example.com')).toBeInTheDocument();
  });

  it('sensitive 字段揭示后显示自动隐藏倒计时（每秒递减），到期自动回到掩码', () => {
    vi.useFakeTimers();
    function HarnessWithReveal() {
      const revealState = useRevealState();
      return (
        <ObjectDetailFieldsList
          fields={[{ key: 'email', value: 'secret@example.com' }]}
          typeId="travel"
          contractTypeId={undefined}
          objFieldDefs={undefined}
          getFieldProperty={(k) =>
            ({ id: k, sensitivityLevel: 'sensitive' }) as unknown as TemplateProperty
          }
          getFieldSensitivity={() => 'sensitive'}
          isFieldDeprecated={() => false}
          getFieldName={(k, label) => label ?? k}
          isRevealed={revealState.isRevealed}
          revealRemainingMs={revealState.revealRemainingMs}
          maskValue={revealState.maskValue}
          handleRevealField={(id) => {
            revealState.reveal(id);
          }}
          handleCopy={vi.fn()}
          copiedField={null}
        />
      );
    }
    render(<HarnessWithReveal />);
    // 掩码态：无倒计时
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    expect(screen.queryByTestId('detail-reveal-countdown')).not.toBeInTheDocument();

    // 揭示态：明文 + 倒计时显示剩余 60s
    fireEvent.click(screen.getByText('common:reveal'));
    expect(screen.queryByText('••••••••')).not.toBeInTheDocument();
    expect(screen.getByText('secret@example.com')).toBeInTheDocument();
    expect(screen.getByTestId('detail-reveal-countdown')).toHaveTextContent('60s');

    // 1 秒后跳动为 59s
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.getByTestId('detail-reveal-countdown')).toHaveTextContent('59s');

    // 1 分钟到期后自动回到掩码态，倒计时消失
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.queryByText('secret@example.com')).not.toBeInTheDocument();
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    expect(screen.queryByTestId('detail-reveal-countdown')).not.toBeInTheDocument();
  });

  it('internal 字段揭示态也不显示倒计时（本就明文展示）', () => {
    vi.useFakeTimers();
    function HarnessInternal() {
      const revealState = useRevealState();
      return (
        <ObjectDetailFieldsList
          fields={[{ key: 'phone', value: '13800138000' }]}
          typeId="travel"
          contractTypeId={undefined}
          objFieldDefs={undefined}
          getFieldProperty={(k) =>
            ({ id: k, sensitivityLevel: 'internal' }) as unknown as TemplateProperty
          }
          getFieldSensitivity={() => 'internal'}
          isFieldDeprecated={() => false}
          getFieldName={(k, label) => label ?? k}
          isRevealed={revealState.isRevealed}
          revealRemainingMs={revealState.revealRemainingMs}
          maskValue={revealState.maskValue}
          handleRevealField={vi.fn()}
          handleCopy={vi.fn()}
          copiedField={null}
        />
      );
    }
    render(<HarnessInternal />);
    // 明文直接可见，无揭示按钮、无倒计时
    expect(screen.getByText('13800138000')).toBeInTheDocument();
    expect(screen.queryByTestId('detail-reveal-countdown')).not.toBeInTheDocument();
  });

  afterEach(() => {
    vi.useRealTimers();
  });
});
