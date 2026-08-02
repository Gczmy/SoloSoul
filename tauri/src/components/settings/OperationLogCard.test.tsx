import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { OperationLogCard } from './OperationLogCard';
import type { AuditLogEntry } from './OperationLogCard';
import type { CustomPage } from '@/stores/settingsStore';

// 用 useTranslation 调用计数探测组件函数体是否真正执行（memo 短路时不执行）。
const renderCount = vi.hoisted(() => vi.fn());
vi.mock('react-i18next', () => ({
  useTranslation: (...args: unknown[]) => {
    renderCount(...args);
    return {
      t: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
      i18n: { language: 'en' },
    };
  },
}));

const entry: AuditLogEntry = {
  id: 1,
  timestamp: '2026-07-01T10:00:00Z',
  actionType: 'object_create',
  entityType: 'object',
  entityId: 'obj_1',
  entityName: 'My Passport',
  performedBy: 'user',
  details: '{"name":"My Passport"}',
};

describe('OperationLogCard', () => {
  beforeEach(() => {
    renderCount.mockClear();
  });

  it('renders action + entity badges from a structured entry', () => {
    render(<OperationLogCard entry={entry} customPages={[]} />);
    expect(screen.getByText('settings:log.action.object_create')).toBeInTheDocument();
    // entity badge 含 `<entity label>: <entityName>`
    expect(screen.getByText(/settings:log\.entity\.object/)).toBeInTheDocument();
  });

  it('renders the formatted detail block when details present', () => {
    const withName: AuditLogEntry = { ...entry, details: 'objectName=My Passport' };
    const { container } = render(<OperationLogCard entry={withName} customPages={[]} />);
    // formatDetail 对无翻译的 action 返回原始 details（key=value 形态），保留原始文本。
    expect(container.textContent).toContain('objectName=My Passport');
  });

  it('skips detail block when details is null', () => {
    render(<OperationLogCard entry={{ ...entry, details: null }} customPages={[]} />);
    expect(document.querySelectorAll('[style*="monospace"]').length).toBe(0);
  });

  it('P218: memo 化——entry 引用不变时组件函数体不执行', () => {
    // 稳定引用：两处必须跨 rerender 保持同一数组/entry，否则 memo 比较必然不等。
    const stableCustomPages: CustomPage[] = [];
    const { rerender } = render(
      <OperationLogCard entry={entry} customPages={stableCustomPages} />,
    );
    // React 18 dev StrictMode 会双调用首次渲染，用相对增量断言。
    const afterMount = renderCount.mock.calls.length;
    // 相同 entry + 相同 customPages 引用 → memo 短路，不执行函数体
    rerender(<OperationLogCard entry={entry} customPages={stableCustomPages} />);
    expect(renderCount.mock.calls.length).toBe(afterMount);
    // 新 entry 引用（内容等价）→ 正常执行
    rerender(<OperationLogCard entry={{ ...entry }} customPages={stableCustomPages} />);
    expect(renderCount.mock.calls.length).toBe(afterMount + 1);
    // customPages 换新引用 → 重新执行
    const newPages = [{ id: 'p', name: 'X', iconId: 'i', createdAt: '2026-01-01', sortOrder: 1 }];
    rerender(<OperationLogCard entry={entry} customPages={newPages} />);
    expect(renderCount.mock.calls.length).toBe(afterMount + 2);
  });

  it('renders system badge when performedBy is system', () => {
    render(<OperationLogCard entry={{ ...entry, performedBy: 'system' }} customPages={[]} />);
    expect(screen.getByText('settings:log.performed_by_system')).toBeInTheDocument();
  });

  it('renders entity name suffix for non-template entities', () => {
    render(<OperationLogCard entry={entry} customPages={[]} />);
    expect(screen.getByText(/settings:log\.entity\.object\s*: My Passport/)).toBeInTheDocument();
  });
});
