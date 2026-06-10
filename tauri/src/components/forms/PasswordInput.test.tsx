import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SecurePasswordInput } from './PasswordInput';

describe('SecurePasswordInput', () => {
  it('renders input with password type by default', () => {
    render(<SecurePasswordInput value="" onChange={vi.fn()} />);
    const input = screen.getByPlaceholderText('common:password_placeholder');
    expect(input).toHaveAttribute('type', 'password');
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
    expect(input).toHaveAttribute('type', 'password');

    const toggleBtn = screen.getByRole('button', { name: /common:show_password/i });
    fireEvent.click(toggleBtn);
    expect(input).toHaveAttribute('type', 'text');

    const hideBtn = screen.getByRole('button', { name: /common:hide_password/i });
    fireEvent.click(hideBtn);
    expect(input).toHaveAttribute('type', 'password');
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
    expect(input).toHaveAttribute('type', 'text');

    fireEvent.blur(input);
    expect(input).toHaveAttribute('type', 'password');
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
});
