import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ErrorBoundary } from './ErrorBoundary';

describe('ErrorBoundary', () => {
  it('子组件正常时直接渲染内容', () => {
    render(
      <ErrorBoundary label="test">
        <div>safe content</div>
      </ErrorBoundary>,
    );
    expect(screen.getByText('safe content')).toBeInTheDocument();
  });

  it('子组件渲染抛错时降级为错误卡片而非白屏，错误信息可见', () => {
    const Bomb = () => {
      throw new Error('boom');
    };
    // React 会向 console.error 输出未捕获信息（边界捕获后），测试预期内
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      render(
        <ErrorBoundary label="test">
          <Bomb />
        </ErrorBoundary>,
      );
      expect(screen.getByText('boom')).toBeInTheDocument();
      expect(screen.getByRole('button')).toBeInTheDocument();
    } finally {
      spy.mockRestore();
    }
  });

  it('重试按钮：子组件恢复后可重新渲染成功内容', () => {
    const bomb = { on: true };
    function Flaky() {
      if (bomb.on) throw new Error('flaky');
      return <div>recovered</div>;
    }
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      render(
        <ErrorBoundary label="test">
          <Flaky />
        </ErrorBoundary>,
      );
      expect(screen.getByText('flaky')).toBeInTheDocument();
      bomb.on = false;
      fireEvent.click(screen.getByRole('button'));
      expect(screen.getByText('recovered')).toBeInTheDocument();
    } finally {
      spy.mockRestore();
    }
  });
});
