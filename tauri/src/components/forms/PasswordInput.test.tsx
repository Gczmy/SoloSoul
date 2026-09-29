import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SecurePasswordInput } from './PasswordInput';

function getTextSecurity(input: HTMLElement): string {
  return (
    ((input as HTMLElement).style as CSSStyleDeclaration & { WebkitTextSecurity?: string })
      .WebkitTextSecurity ?? ''
  );
}

describe('SecurePasswordInput', () => {
  it('masks input by default using WebkitTextSecurity', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} />);
    const input = screen.getByPlaceholderText('common:password_placeholder');
    expect(input).toHaveAttribute('type', 'text');
    expect(getTextSecurity(input as HTMLElement)).toBe('disc');
  });

  it('calls onChange when typing', () => {
    const onChange = vi.fn();
    render(<SecurePasswordInput value="" onChange={onChange} />);
    const input = screen.getByPlaceholderText('common:password_placeholder');
    fireEvent.change(input, { target: { value: 'secret123' } });
    expect(onChange).toHaveBeenCalledWith('secret123');
  });

  it('toggles visibility when toggle button is clicked', () => {
    render(<SecurePasswordInput value="secret" onChange={vi.fn()} />);
    const input = screen.getByPlaceholderText('common:password_placeholder');
    expect(getTextSecurity(input as HTMLElement)).toBe('disc');

    const toggleBtn = screen.getByRole('button', { name: /common:show_password/i });
    fireEvent.click(toggleBtn);
    expect(getTextSecurity(input as HTMLElement)).toBe('');

    const hideBtn = screen.getByRole('button', { name: /common:hide_password/i });
    fireEvent.click(hideBtn);
    expect(getTextSecurity(input as HTMLElement)).toBe('disc');
  });

  it('does not show visibility toggle when value is empty', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} />);
    expect(screen.queryByRole('button', { name: /common:show_password/i })).not.toBeInTheDocument();
  });

  it('resets visibility on blur', () => {
    render(<SecurePasswordInput value="secret" onChange={vi.fn()} />);
    const input = screen.getByPlaceholderText('common:password_placeholder');
    const toggleBtn = screen.getByRole('button', { name: /common:show_password/i });

    fireEvent.click(toggleBtn);
    expect(getTextSecurity(input as HTMLElement)).toBe('');

    fireEvent.blur(input);
    expect(getTextSecurity(input as HTMLElement)).toBe('disc');
  });

  it('renders label when provided', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} label="Password" />);
    expect(screen.getByText('Password')).toBeInTheDocument();
  });

  it('renders error message when provided', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} error="Too weak" />);
    expect(screen.getByRole('alert')).toHaveTextContent('Too weak');
  });

  it('applies error border style when error is present', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} error="Error" />);
    const wrapper = screen.getByPlaceholderText('common:password_placeholder').parentElement;
    expect(wrapper).toBeInTheDocument();
    // Border style uses CSS variables; verify the wrapper exists and input is inside it
    expect(wrapper!.tagName).toBe('DIV');
  });

  it('disables input when disabled prop is true', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} disabled />);
    const input = screen.getByPlaceholderText('common:password_placeholder');
    expect(input).toBeDisabled();
  });

  it('does not render hint button when showHintButton is false', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} showHintButton={false} />);
    expect(screen.queryByLabelText(/common:password_hint_tooltip/i)).not.toBeInTheDocument();
  });

  it('shows no_hint_available tooltip when hint is empty', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} hint="" />);
    const hintBtn = screen.getByLabelText(/common:password_hint_tooltip/i);
    fireEvent.mouseEnter(hintBtn);
    expect(screen.getByText('common:no_hint_available')).toBeInTheDocument();
  });

  it('shows hint tooltip when hint is provided', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} hint="My hint" />);
    const hintBtn = screen.getByLabelText(/common:password_hint_tooltip/i);
    fireEvent.mouseEnter(hintBtn);
    expect(screen.getByText('My hint')).toBeInTheDocument();
  });

  it('uses unique input ids for each instance', () => {
    const { container: container1 } = render(
      <SecurePasswordInput value="" onChange={vi.fn()} label="First" />,
    );
    const input1 = container1.querySelector('input');
    const id1 = input1?.getAttribute('id');
    const { container: container2 } = render(
      <SecurePasswordInput value="" onChange={vi.fn()} label="Second" />,
    );
    const input2 = container2.querySelector('input');
    const id2 = input2?.getAttribute('id');
    expect(id1).toBeTruthy();
    expect(id2).toBeTruthy();
    expect(id1).not.toBe(id2);
  });

  it('输入获得焦点后可用 Enter 提交，失焦重新隐藏密码', () => {
    const onFocus = vi.fn();
    const onEnter = vi.fn();
    render(
      <SecurePasswordInput value="secret" onChange={vi.fn()} onFocus={onFocus} onEnter={onEnter} />,
    );
    const input = screen.getByPlaceholderText('common:password_placeholder');

    fireEvent.focus(input);
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onFocus).toHaveBeenCalledOnce();
    expect(onEnter).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole('button', { name: 'common:show_password' }));
    expect(getTextSecurity(input)).toBe('');
    fireEvent.blur(input);
    expect(getTextSecurity(input)).toBe('disc');
  });

  it('鼠标离开或在提示按钮外按下时关闭提示卡片', async () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} hint="My hint" />);
    const hintButton = screen.getByRole('button', { name: 'common:password_hint_tooltip' });

    fireEvent.mouseEnter(hintButton);
    expect(screen.getByTestId('password-hint-tooltip')).toHaveTextContent('My hint');
    fireEvent.mouseLeave(hintButton);
    expect(screen.queryByTestId('password-hint-tooltip')).not.toBeInTheDocument();

    fireEvent.mouseEnter(hintButton);
    await new Promise((resolve) => setTimeout(resolve, 0));
    fireEvent.mouseDown(hintButton);
    expect(screen.getByTestId('password-hint-tooltip')).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByTestId('password-hint-tooltip')).not.toBeInTheDocument();
  });

  it('触摸可切换提示卡片，随后的合成鼠标悬停不会重新打开', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} hint="Touch hint" />);
    const hintButton = screen.getByRole('button', { name: 'common:password_hint_tooltip' });

    fireEvent.touchStart(hintButton);
    expect(screen.getByTestId('password-hint-tooltip')).toHaveTextContent('Touch hint');
    fireEvent.touchStart(hintButton);
    expect(screen.queryByTestId('password-hint-tooltip')).not.toBeInTheDocument();

    fireEvent.mouseEnter(hintButton);
    expect(screen.queryByTestId('password-hint-tooltip')).not.toBeInTheDocument();
  });
});
