import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ObjectDetailFieldsList } from './ObjectDetailFieldsList';
import type { ObjectDetailFieldEntry } from './objectDetailUtils';
import { useRevealState } from '@/hooks/useRevealState';
import type { SensitivityLevel, TemplateProperty } from '@/types/template';

// 使用真实 useRevealState（含真实 maskValue 逻辑），验证详情卡片掩码规则：
// - public 明文；其他等级默认掩码；
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
  fields: ObjectDetailFieldEntry[];
  sensitivities: Record<string, SensitivityLevel>;
}) {
  return (
    <ObjectDetailFieldsList
      objectId="object"
      accountId="account"
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
      handleRevealField={vi.fn()}
      handleCopy={vi.fn()}
      copiedField={null}
    />
  );
}

describe('ObjectDetailFieldsList 掩码规则', () => {
  it('internal 字段：默认掩码并提供揭示按钮', () => {
    render(
      <Harness
        fields={[{ kind: 'field' as const, key: 'phone', value: '13800138000' }]}
        sensitivities={{ phone: 'internal' }}
      />,
    );
    expect(screen.queryByText('13800138000')).not.toBeInTheDocument();
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    expect(screen.getByText('common:reveal')).toBeInTheDocument();
  });

  it('public 字段：直接显示明文，无揭示按钮', () => {
    render(
      <Harness
        fields={[{ kind: 'field' as const, key: 'nickname', value: 'Alice' }]}
        sensitivities={{ nickname: 'public' }}
      />,
    );
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.queryByText('••••••••')).not.toBeInTheDocument();
  });

  it('sensitive 字段：掩码为 8 圆点 + 显示揭示按钮', () => {
    render(
      <Harness
        fields={[{ kind: 'field' as const, key: 'email', value: 'secret@example.com' }]}
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
        fields={[{ kind: 'field' as const, key: 'password', value: 'p@ss' }]}
        sensitivities={{ password: 'critical' }}
      />,
    );
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    expect(screen.queryByText('p@ss')).not.toBeInTheDocument();
    expect(screen.getByText('common:unlock')).toBeInTheDocument();
  });

  it('sensitive 字段点击揭示后显示明文（reveal 链路完整）', async () => {
    // 模拟 modal 层 handleRevealField 的真实行为：非 critical 直接 reveal
    let capturedId: string | null = null;
    function HarnessWithReveal() {
      return (
        <ObjectDetailFieldsList
          objectId="object"
          accountId="account"
          fields={[{ kind: 'field' as const, key: 'email', value: 'secret@example.com' }]}
          typeId="travel"
          contractTypeId={undefined}
          objFieldDefs={undefined}
          getFieldProperty={(k) =>
            ({ id: k, sensitivityLevel: 'sensitive' }) as unknown as TemplateProperty
          }
          getFieldSensitivity={() => 'sensitive'}
          isFieldDeprecated={() => false}
          getFieldName={(k, label) => label ?? k}
          handleRevealField={async (id) => {
            capturedId = id;
            return true;
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
    await act(async () => {
      fireEvent.click(screen.getByText('common:reveal'));
    });
    expect(capturedId).toBe('travel.email');
    expect(screen.queryByText('••••••••')).not.toBeInTheDocument();
    expect(screen.getByText('secret@example.com')).toBeInTheDocument();
  });

  it('sensitive 字段揭示后显示自动隐藏倒计时（每秒递减），到期自动回到掩码', async () => {
    vi.useFakeTimers();
    function HarnessWithReveal() {
      return (
        <ObjectDetailFieldsList
          objectId="object"
          accountId="account"
          fields={[{ kind: 'field' as const, key: 'email', value: 'secret@example.com' }]}
          typeId="travel"
          contractTypeId={undefined}
          objFieldDefs={undefined}
          getFieldProperty={(k) =>
            ({ id: k, sensitivityLevel: 'sensitive' }) as unknown as TemplateProperty
          }
          getFieldSensitivity={() => 'sensitive'}
          isFieldDeprecated={() => false}
          getFieldName={(k, label) => label ?? k}
          handleRevealField={async () => {
            return true;
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
    await act(async () => {
      fireEvent.click(screen.getByText('common:reveal'));
    });
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

  it('internal 未揭示时无倒计时', () => {
    vi.useFakeTimers();
    function HarnessInternal() {
      return (
        <ObjectDetailFieldsList
          objectId="object"
          accountId="account"
          fields={[{ kind: 'field' as const, key: 'phone', value: '13800138000' }]}
          typeId="travel"
          contractTypeId={undefined}
          objFieldDefs={undefined}
          getFieldProperty={(k) =>
            ({ id: k, sensitivityLevel: 'internal' }) as unknown as TemplateProperty
          }
          getFieldSensitivity={() => 'internal'}
          isFieldDeprecated={() => false}
          getFieldName={(k, label) => label ?? k}
          handleRevealField={vi.fn()}
          handleCopy={vi.fn()}
          copiedField={null}
        />
      );
    }
    render(<HarnessInternal />);
    // 未揭示不显示倒计时。
    expect(screen.queryByText('13800138000')).not.toBeInTheDocument();
    expect(screen.queryByTestId('detail-reveal-countdown')).not.toBeInTheDocument();
  });

  afterEach(() => {
    vi.useRealTimers();
  });
});

describe('动态字段组树状渲染（与历史快照同构）', () => {
  const groupEntry: ObjectDetailFieldEntry = {
    kind: 'dynamicGroup',
    key: '__dynamic_group__',
    type: 'dynamic_group',
    children: [
      { label: '备注一', value: 'hello world', type: 'text' },
      { label: '备注二', value: 'second value', type: 'text' },
    ],
  };

  function renderGroup(sensitivities: Record<string, SensitivityLevel>) {
    return render(<Harness fields={[groupEntry]} sensitivities={sensitivities} />);
  }

  it('组头仅显示一次敏感度徽章；子行不重复显示', () => {
    renderGroup({ __dynamic_group__: 'internal' });
    // internal 子项默认占位。
    expect(screen.queryByText('hello world')).not.toBeInTheDocument();
    expect(screen.queryByText('second value')).not.toBeInTheDocument();
    // 子行名称可见
    expect(screen.getByText('备注一')).toBeInTheDocument();
    expect(screen.getByText('备注二')).toBeInTheDocument();
    // 组名回退为本地化「动态字段组」
    expect(screen.getByText(/动态字段组/)).toBeInTheDocument();
    // 敏感度徽章恰好一个（组头），子行不重复——按徽章文本匹配计数
    const badges = screen.getAllByText(/internal/);
    expect(badges.length).toBe(1);
  });

  it('sensitive 组：子行值随组掩码，揭示按钮仅组头一个', () => {
    renderGroup({ __dynamic_group__: 'sensitive' });
    // 两个子行值均随组掩码
    expect(screen.getAllByText('••••••••').length).toBe(2);
    expect(screen.queryByText('hello world')).not.toBeInTheDocument();
    expect(screen.queryByText('second value')).not.toBeInTheDocument();
    // 揭示按钮仅组头一个
    expect(screen.getAllByText('common:reveal').length).toBe(1);
  });
});

describe('复制先揭示/验证', () => {
  it('public 组保留子项等级，整组及子项复制不能绕过最高保护等级', async () => {
    const copy = vi.fn(),
      authorize = vi.fn().mockResolvedValue(false);
    const { container } = render(
      <ObjectDetailFieldsList
        objectId="group-object"
        accountId="a"
        typeId="identity"
        fields={[
          {
            kind: 'dynamicGroup',
            key: 'group',
            children: [
              { id: 'open', label: 'Open', value: 'PUBLIC_VALUE', sensitivityLevel: 'public' },
              {
                id: 'protected',
                label: 'Protected',
                value: 'CRITICAL_VALUE',
                sensitivityLevel: 'critical',
              },
              { id: 'unknown', label: 'Unknown', value: 'UNKNOWN_VALUE' },
            ],
          },
        ]}
        getFieldProperty={() => undefined}
        getFieldSensitivity={() => 'public'}
        isFieldDeprecated={() => false}
        getFieldName={() => 'Group'}
        handleRevealField={authorize}
        handleCopy={copy}
        copiedField={null}
      />,
    );
    expect(screen.getByText('PUBLIC_VALUE')).toBeInTheDocument();
    expect(container.innerHTML).not.toContain('CRITICAL_VALUE');
    expect(container.innerHTML).not.toContain('UNKNOWN_VALUE');
    expect(screen.getByText('common:unlock')).toBeInTheDocument();
    await act(async () => fireEvent.click(screen.getAllByText('common:copy')[0]));
    expect(authorize).toHaveBeenCalledWith('identity.group', 'critical', 'Group');
    expect(copy).not.toHaveBeenCalled();
  });
  function setup(
    sens: SensitivityLevel,
    verify: () => Promise<boolean>,
    copy: (v: string, key: string) => void,
  ) {
    function CopyHarness() {
      const state = useRevealState();
      return (
        <ObjectDetailFieldsList
          objectId="object"
          accountId="account"
          fields={[{ kind: 'field', key: 'secret', value: 'actual text' }]}
          typeId="identity"
          getFieldProperty={() => undefined}
          getFieldSensitivity={() => sens}
          isFieldDeprecated={() => false}
          getFieldName={() => 'Secret'}
          handleRevealField={async (id) => {
            if (!(await verify())) return false;
            state.reveal(id);
            return true;
          }}
          handleCopy={copy}
          copiedField={null}
        />
      );
    }
    render(<CopyHarness />);
  }
  it('敏感字段先显示明文再复制原始内容', async () => {
    const verify = vi.fn().mockResolvedValue(true),
      copy = vi.fn();
    setup('sensitive', verify, copy);
    await act(async () => fireEvent.click(screen.getByText('common:copy')));
    expect(verify).toHaveBeenCalledOnce();
    expect(screen.getByText('actual text')).toBeInTheDocument();
    expect(copy).toHaveBeenCalledExactlyOnceWith('actual text', 'secret');
  });
  it.each([false, true])('关键字段验证结果=%s；等待期间不复制', async (ok) => {
    let finish!: (ok: boolean) => void;
    const verify = vi.fn(
        () =>
          new Promise<boolean>((resolve) => {
            finish = resolve;
          }),
      ),
      copy = vi.fn();
    setup('critical', verify, copy);
    fireEvent.click(screen.getByText('common:copy'));
    expect(copy).not.toHaveBeenCalled();
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    await act(async () => finish(ok));
    expect(copy).toHaveBeenCalledTimes(ok ? 1 : 0);
    if (ok) expect(copy).toHaveBeenCalledWith('actual text', 'secret');
    else expect(screen.getByText('••••••••')).toBeInTheDocument();
  });
});
