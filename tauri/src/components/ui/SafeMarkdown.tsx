import ReactMarkdown from 'react-markdown';
import type { Options as MarkdownOptions } from 'react-markdown';

interface SafeMarkdownProps extends MarkdownOptions {
  className?: string;
}

export function SafeMarkdown({ className, ...props }: SafeMarkdownProps) {
  return (
    <div className={className}>
      <ReactMarkdown
        {...props}
        disallowedElements={['script', 'style', 'iframe', 'object', 'embed']}
        unwrapDisallowed
      />
    </div>
  );
}
