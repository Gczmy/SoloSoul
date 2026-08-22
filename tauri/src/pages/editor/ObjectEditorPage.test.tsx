import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { ObjectEditorPage } from './ObjectEditorPage';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import type { UserTemplate } from '@/types/template';

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="page-shell" data-title={title}>
      {children}
    </div>
  ),
}));

// 覆盖 setup.ts 的 react-i18next mock：其每次渲染返回新的 t 函数，导致
// 编辑流程中依赖 t 的加载 effect 在每次渲染后重新触发 → 无限 getObject 循环。
// 这里返回模块级稳定 t（与真实 react-i18next 的 useTranslation 行为一致）。
const { stableT } = vi.hoisted(() => ({
  stableT: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: stableT,
    i18n: { language: 'en', changeLanguage: vi.fn(() => Promise.resolve()) },
  }),
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: vi.fn(),
    useParams: vi.fn(),
    useSearchParams: vi.fn(),
  };
});

const template: UserTemplate = {
  id: 'tpl1',
  accountId: 'acc1',
  name: '测试模板',
  iconId: undefined,
  category: 'identity',
  createdAt: '2026-08-22T00:00:00Z',
  updatedAt: '2026-08-22T00:00:00Z',
  properties: [
    { id: 'birthDate', name: '出生日期', type: 'date' },
    { id: 'meetTime', name: '会议时间', type: 'datetime' },
  ],
};

describe('ObjectEditorPage datetime save validation', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(navigate);
    vi.mocked(useParams).mockReturnValue({});
    vi.mocked(useSearchParams).mockReturnValue([new URLSearchParams(), vi.fn()]);
    useAuthStore.setState({ currentAccount: { id: 'acc1', name: 'Acc' } });
    useTemplateStore.setState({
      templates: [template],
      loadTemplates: vi.fn().mockResolvedValue(undefined),
    });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'object_field_suggestions') return [];
      if (cmd === 'object_create') {
        return {
          id: 'obj1',
          accountId: 'acc1',
          name: 'x',
          typeId: 'identity',
          properties: {},
          sensitivityLevel: 'internal',
          createdAt: '2026-08-22T00:00:00Z',
          updatedAt: '2026-08-22T00:00:00Z',
        };
      }
      return undefined;
    });
  });

  it('fills only the date field and saves without an error on the empty datetime field', async () => {
    render(
      <MemoryRouter>
        <ObjectEditorPage />
      </MemoryRouter>,
    );

    // 等模板加载、表单渲染出两个 DatePicker（日期 + 日期时间）
    await waitFor(() => {
      expect(screen.getAllByLabelText('年份输入').length).toBeGreaterThanOrEqual(2);
    });

    // 只填写第一个（日期）字段
    fireEvent.change(screen.getAllByLabelText('年份输入')[0], { target: { value: '2024' } });
    fireEvent.change(screen.getAllByLabelText('月份输入')[0], { target: { value: '12' } });
    fireEvent.change(screen.getAllByLabelText('日期输入')[0], { target: { value: '31' } });

    // 日期时间字段保持空
    expect((screen.getAllByLabelText('年份输入')[1] as HTMLInputElement).value).toBe('');

    // 点保存
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));

    // 不应出现日期时间校验错误
    await waitFor(() => {
      expect(screen.queryByText('editor:validation_datetime')).not.toBeInTheDocument();
    });
    // 保存应成功（object_create 被调用），且日期值确实进入 properties
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith(
        'object_create',
        expect.objectContaining({
          input: expect.objectContaining({
            templateId: 'tpl1',
            properties: expect.objectContaining({ birthDate: '2024-12-31' }),
          }),
        }),
      );
    });
  });

  it('partial-year input (12-12-12) is flagged at save instead of silently dropped', async () => {
    render(
      <MemoryRouter>
        <ObjectEditorPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getAllByLabelText('年份输入').length).toBeGreaterThanOrEqual(2);
    });

    // 用户填满所有分段但年份不足 4 位（12-12-12）：此前静默丢值、保存「成功」却无字段
    fireEvent.change(screen.getAllByLabelText('年份输入')[0], { target: { value: '12' } });
    fireEvent.change(screen.getAllByLabelText('月份输入')[0], { target: { value: '12' } });
    fireEvent.change(screen.getAllByLabelText('日期输入')[0], { target: { value: '12' } });
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));

    // 保存必须报错（字段下可见错误），而不是静默丢弃后保存成功
    await waitFor(() => {
      expect(screen.getByText('editor:validation_date')).toBeInTheDocument();
    });
    // object_create 不得被调用（校验失败，不落库空对象）
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith('object_create', expect.anything());
  });

  it('single-digit date input (2024-1-5) is padded and actually saved', async () => {
    render(
      <MemoryRouter>
        <ObjectEditorPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getAllByLabelText('年份输入').length).toBeGreaterThanOrEqual(2);
    });

    // 中文用户常见写法：单数字月份/日期（2024-1-5）——此前永不提交，保存静默丢值
    fireEvent.change(screen.getAllByLabelText('年份输入')[0], { target: { value: '2024' } });
    fireEvent.change(screen.getAllByLabelText('月份输入')[0], { target: { value: '1' } });
    fireEvent.change(screen.getAllByLabelText('日期输入')[0], { target: { value: '5' } });

    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));

    // 补零后的日期值必须真正进入 object_create 载荷
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith(
        'object_create',
        expect.objectContaining({
          input: expect.objectContaining({
            properties: expect.objectContaining({ birthDate: '2024-01-05' }),
          }),
        }),
      );
    });
  });

  it('does not error on the datetime field after an invalid draft was typed and then cleared', async () => {
    render(
      <MemoryRouter>
        <ObjectEditorPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getAllByLabelText('年份输入').length).toBeGreaterThanOrEqual(2);
    });

    // 在日期时间字段（第二个 DatePicker）输入非法完整草稿
    fireEvent.change(screen.getAllByLabelText('年份输入')[1], { target: { value: '2024' } });
    fireEvent.change(screen.getAllByLabelText('月份输入')[1], { target: { value: '02' } });
    fireEvent.change(screen.getAllByLabelText('日期输入')[1], { target: { value: '30' } });
    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '25' } });
    fireEvent.change(screen.getByLabelText('分钟输入'), { target: { value: '99' } });

    // 清空日期时间字段（删掉年份段 → 未输满 → 撤销草稿）
    fireEvent.change(screen.getAllByLabelText('年份输入')[1], { target: { value: '' } });
    expect((screen.getAllByLabelText('年份输入')[1] as HTMLInputElement).value).toBe('');

    // 填写日期字段
    fireEvent.change(screen.getAllByLabelText('年份输入')[0], { target: { value: '2024' } });
    fireEvent.change(screen.getAllByLabelText('月份输入')[0], { target: { value: '12' } });
    fireEvent.change(screen.getAllByLabelText('日期输入')[0], { target: { value: '31' } });

    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));

    // 不应出现日期时间校验错误，保存应成功
    await waitFor(() => {
      expect(screen.queryByText('editor:validation_datetime')).not.toBeInTheDocument();
    });
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith(
        'object_create',
        expect.objectContaining({ input: expect.objectContaining({ templateId: 'tpl1' }) }),
      );
    });
  });

  it('clears the field after a failed save and saving again succeeds without error', async () => {
    render(
      <MemoryRouter>
        <ObjectEditorPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getAllByLabelText('年份输入').length).toBeGreaterThanOrEqual(2);
    });

    // 日期时间字段输入不可能日期 → 保存报错
    fireEvent.change(screen.getAllByLabelText('年份输入')[1], { target: { value: '2024' } });
    fireEvent.change(screen.getAllByLabelText('月份输入')[1], { target: { value: '02' } });
    fireEvent.change(screen.getAllByLabelText('日期输入')[1], { target: { value: '30' } });
    fireEvent.change(screen.getByLabelText('小时输入'), { target: { value: '10' } });
    fireEvent.change(screen.getByLabelText('分钟输入'), { target: { value: '30' } });
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));
    await waitFor(() => {
      expect(screen.queryByText('editor:validation_datetime')).toBeInTheDocument();
    });

    // 清空日期时间字段（删掉年份段）
    fireEvent.change(screen.getAllByLabelText('年份输入')[1], { target: { value: '' } });

    // 再次保存应成功，不再报错
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));
    await waitFor(() => {
      expect(screen.queryByText('editor:validation_datetime')).not.toBeInTheDocument();
    });
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith(
        'object_create',
        expect.objectContaining({ input: expect.objectContaining({ templateId: 'tpl1' }) }),
      );
    });
  });

  it('edit page shows a stored normal date value (identity template scenario)', async () => {
    vi.mocked(useParams).mockReturnValue({ objectId: 'obj-date' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'object_field_suggestions') return [];
      if (cmd === 'object_get') {
        return {
          id: 'obj-date',
          accountId: 'acc1',
          name: '张三',
          typeId: 'identity',
          templateId: 'tpl1',
          templateType: 'user',
          properties: {
            // 新建对象保存后回读的真实存量值：正常日期，敏感度 internal
            birthDate: '2024-12-31',
            __fields: {
              birthDate: { name: '出生日期', type: 'date' },
              meetTime: { name: '会议时间', type: 'datetime' },
            },
          },
          sensitivityLevel: 'internal',
          createdAt: '2026-08-22T00:00:00Z',
          updatedAt: '2026-08-22T00:00:00Z',
        };
      }
      return undefined;
    });

    render(
      <MemoryRouter>
        <ObjectEditorPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('张三')).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getAllByLabelText('年份输入').length).toBeGreaterThanOrEqual(2);
    });

    // 存量正常日期值必须回显在分段中（用户场景：创建后再次编辑不应看到空字段）
    expect((screen.getAllByLabelText('年份输入')[0] as HTMLInputElement).value).toBe('2024');
    expect((screen.getAllByLabelText('月份输入')[0] as HTMLInputElement).value).toBe('12');
    expect((screen.getAllByLabelText('日期输入')[0] as HTMLInputElement).value).toBe('31');
  });

  it('shows a stored unparseable datetime value visibly, and clearing it allows saving', async () => {
    vi.mocked(useParams).mockReturnValue({ objectId: 'obj1' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'object_field_suggestions') return [];
      if (cmd === 'object_get') {
        return {
          id: 'obj1',
          accountId: 'acc1',
          name: '已有对象',
          typeId: 'identity',
          templateId: 'tpl1',
          templateType: 'user',
          properties: {
            // 存量脏数据：不可能日期，此前 DatePicker 无法解析 → 字段看似为空
            meetTime: '2024-02-30T10:30',
          },
          sensitivityLevel: 'internal',
          createdAt: '2026-08-22T00:00:00Z',
          updatedAt: '2026-08-22T00:00:00Z',
        };
      }
      if (cmd === 'object_update') return undefined;
      return undefined;
    });

    render(
      <MemoryRouter>
        <ObjectEditorPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('已有对象')).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getAllByLabelText('年份输入').length).toBeGreaterThanOrEqual(2);
    });

    // 存量不可能日期不再「看似为空」：字段显示原始值，用户可看到并修正/清空
    expect((screen.getAllByLabelText('年份输入')[1] as HTMLInputElement).value).toBe('2024');
    expect((screen.getAllByLabelText('日期输入')[1] as HTMLInputElement).value).toBe('30');

    // 清空日期时间字段（删掉年份段 → 撤销存量脏数据）
    fireEvent.change(screen.getAllByLabelText('年份输入')[1], { target: { value: '' } });

    // 填写日期字段后保存
    fireEvent.change(screen.getAllByLabelText('年份输入')[0], { target: { value: '2024' } });
    fireEvent.change(screen.getAllByLabelText('月份输入')[0], { target: { value: '12' } });
    fireEvent.change(screen.getAllByLabelText('日期输入')[0], { target: { value: '31' } });
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));

    // 清空后保存不应报错
    await waitFor(() => {
      expect(screen.queryByText('editor:validation_datetime')).not.toBeInTheDocument();
    });
    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith('object_update', expect.anything());
    });
  });
});
