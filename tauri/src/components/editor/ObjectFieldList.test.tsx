import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ObjectFieldList } from './ObjectFieldList';

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
});
