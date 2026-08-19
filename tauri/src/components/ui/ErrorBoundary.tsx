import { Component, type ErrorInfo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { logger } from '@/lib/logger';

interface ErrorBoundaryProps {
  children: ReactNode;
  /** 出错区域的名称，用于日志定位（如页面名） */
  label?: string;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

/**
 * 页面级错误边界：子组件渲染异常时不再整树卸载（应用无 ErrorBoundary 时，
 * React 会卸载整棵组件树 → 白屏「所有内容消失」），而是降级为可恢复的
 * 错误卡片（含重试按钮），导航/壳层保持可用。
 *
 * 用法：包裹路由 element 或页面内容，按最小粒度（单页）划分即可。
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    logger.error(
      `[ErrorBoundary]${this.props.label ? ` ${this.props.label}` : ''} render failed:`,
      error,
      info.componentStack,
    );
  }

  handleRetry = () => {
    // 清空错误态重渲染；若子组件再次抛错会被边界再次捕获（不会白屏）
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return <ErrorFallback error={this.state.error} onRetry={this.handleRetry} />;
    }
    return this.props.children;
  }
}

function ErrorFallback({ error, onRetry }: { error: Error | null; onRetry: () => void }) {
  const { t } = useTranslation(['common']);
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 12,
        padding: '48px 24px',
        textAlign: 'center',
      }}
    >
      <div style={{ fontSize: 'var(--text-body)', color: 'var(--text-primary)' }}>
        {t('common:page_render_error', { defaultValue: '页面渲染出错，请重试' })}
      </div>
      {error && (
        <div
          style={{
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            maxWidth: 320,
            wordBreak: 'break-word',
            lineHeight: 1.5,
          }}
        >
          {String(error.message || error)}
        </div>
      )}
      <button
        type="button"
        onClick={onRetry}
        className="interactive-toolbar"
        style={{
          padding: '8px 16px',
          borderRadius: 8,
          borderWidth: 1,
          borderStyle: 'solid',
          fontSize: 'var(--text-body-sm)',
          fontWeight: 500,
          fontFamily: 'inherit',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          gap: 6,
        }}
      >
        <RefreshCw size={14} />
        {t('common:retry', { defaultValue: '重试' })}
      </button>
    </div>
  );
}
