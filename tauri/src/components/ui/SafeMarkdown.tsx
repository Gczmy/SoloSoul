import ReactMarkdown from 'react-markdown';
import type { Components } from 'react-markdown';

interface SafeMarkdownProps {
  children: string;
  /** Optional custom components for ReactMarkdown */
  components?: Components;
  /** Optional remark plugins (pass-through to ReactMarkdown) */
  remarkPlugins?: any[];
  /** Optional rehype plugins (pass-through to ReactMarkdown) */
  rehypePlugins?: any[];
  /** Optional URL transform function */
  urlTransform?: (url: string) => string;
  /** Optional class name for the container */
  className?: string;
}

/**
 * Safe wrapper around ReactMarkdown with XSS protection.
 *
 * - Disallows dangerous HTML elements: script, style, iframe, object, embed
 * - react-markdown v10+ already escapes raw HTML by default (renders as text, not executed)
 * - This provides defense-in-depth for LLM-rendered content and external data
 */
export function SafeMarkdown({
  children,
  components,
  remarkPlugins,
  rehypePlugins,
  urlTransform,
  className,
}: SafeMarkdownProps) {
  return (
    <div className={className}>
      <ReactMarkdown
        components={components}
        remarkPlugins={remarkPlugins}
        rehypePlugins={rehypePlugins}
        urlTransform={urlTransform}
        disallowedElements={['script', 'style', 'iframe', 'object', 'embed']}
        unwrapDisallowed
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}
