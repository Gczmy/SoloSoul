import type { CSSProperties } from 'react';
import ReactMarkdown from 'react-markdown';
import type { Options as MarkdownOptions } from 'react-markdown';

interface SafeMarkdownProps extends MarkdownOptions {
  className?: string;
  /** 应用到外层包裹 div 的样式（ReactMarkdown 本身不接受 style） */
  style?: CSSProperties;
}

export function SafeMarkdown({ className, style, ...props }: SafeMarkdownProps) {
  return (
    <div className={className} style={style}>
      <ReactMarkdown
        {...props}
        disallowedElements={['script', 'style', 'iframe', 'object', 'embed']}
        unwrapDisallowed
      />
    </div>
  );
}
