import { describe, it, expect, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { ObjectFieldList } from './ObjectFieldList';
import type { ObjectData } from '@/stores/objectStore';

const datetimeField = {
  key: 'birthday',
  label: '出生日期',
  type: 'datetime',
  sensitivityLevel: 'internal',
};

describe('ObjectFieldList', () => {
  it('renders a save-time validation error under the corresponding field', () => {
    render(
      <ObjectFieldList
        fields={[datetimeField]}
        displayFields={[datetimeField]}
        values={{ birthday: '2024-02-30' }}
        onChange={vi.fn()}
        validationErrors={{ birthday: '请输入有效日期时间（YYYY-MM-DD HH:MM）' }}
        onClearError={vi.fn()}
        currentObject={null}
        getSensitivity={() => 'internal'}
        isNew
        suggestions={{}}
      />,
    );

    // 错误文本渲染在字段下方
    expect(screen.getByText('请输入有效日期时间（YYYY-MM-DD HH:MM）')).toBeInTheDocument();
  });

  it('renders no error text when the field has no validation error', () => {
    render(
      <ObjectFieldList
        fields={[datetimeField]}
        displayFields={[datetimeField]}
        values={{ birthday: '2024-02-29' }}
        onChange={vi.fn()}
        validationErrors={{}}
        onClearError={vi.fn()}
        currentObject={null}
        getSensitivity={() => 'internal'}
        isNew
        suggestions={{}}
      />,
    );

    expect(screen.queryByText(/有效日期时间/)).not.toBeInTheDocument();
  });

  it('recovers a deleted template field without accepting an invalid sensitivity label', () => {
    const currentObject: ObjectData = {
      id: 'obj-1',
      accountId: 'acc-1',
      name: '旧对象',
      typeId: 'identity',
      properties: {
        __fields: {
          secret: { name: '旧字段', type: 'text' },
          pin: { name: '关键字段', type: 'text' },
        },
        secret: '原值',
        pin: '1234',
      },
      sensitivityLevel: 'internal',
      propertyLabels: { secret: 'invalid-level', pin: 'critical' },
      createdAt: '2026-08-22T00:00:00Z',
      updatedAt: '2026-08-22T00:00:00Z',
    };
    const onChange = vi.fn();
    const onClearError = vi.fn();
    render(
      <ObjectFieldList
        fields={[]}
        displayFields={[]}
        values={{ secret: '原值', pin: '1234', __fields: currentObject.properties.__fields }}
        onChange={onChange}
        validationErrors={{ secret: '请输入内容' }}
        onClearError={onClearError}
        currentObject={currentObject}
        getSensitivity={() => 'internal'}
        isNew={false}
      />,
    );

    expect(screen.getByText('旧字段')).toBeInTheDocument();
    expect(screen.queryByText('__fields')).toBeNull();
    expect(screen.getByTitle('sensitivity_label: internal')).toBeInTheDocument();
    expect(screen.getByTitle('sensitivity_label: critical')).toBeInTheDocument();
    fireEvent.change(screen.getByDisplayValue('原值'), { target: { value: '修正值' } });
    expect(onChange).toHaveBeenCalledWith('secret', '修正值');
    expect(onClearError).toHaveBeenCalledWith('secret');
  });
});
