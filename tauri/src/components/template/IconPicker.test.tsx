import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IconPicker } from './IconPicker';

describe('IconPicker', () => {
  it('非法图标 ID 显示默认选择，分类过滤后只提交被点击的图标', () => {
    const onChange = vi.fn();
    render(<IconPicker value="toString" onChange={onChange} />);

    expect(screen.getByTitle('document')).toHaveClass('selected-accent');
    fireEvent.click(screen.getByRole('button', { name: 'icon_category_security' }));
    expect(screen.queryByTitle('document')).not.toBeInTheDocument();
    expect(screen.getByTitle('shield')).toBeInTheDocument();

    fireEvent.click(screen.getByTitle('shield'));
    expect(onChange).toHaveBeenCalledExactlyOnceWith('shield');
  });
});
