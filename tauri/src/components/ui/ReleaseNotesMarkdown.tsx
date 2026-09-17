import type { CSSProperties } from 'react';
import type { Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { SafeMarkdown } from './SafeMarkdown';
import styles from './ReleaseNotesMarkdown.module.css';

interface ReleaseNotesMarkdownProps {
  children: string;
  className?: string;
  style?: CSSProperties;
}

const components: Components = {
  table: ({ node: _node, ...props }) => (
    <div className={styles.tableScroll}>
      <table {...props} />
    </div>
  ),
};

/** 三个更新说明入口共用 GFM 渲染；不启用原始 HTML，保留安全链接协议过滤。 */
export function ReleaseNotesMarkdown({ children, className, style }: ReleaseNotesMarkdownProps) {
  return (
    <SafeMarkdown
      className={['release-notes-md', styles.notes, className].filter(Boolean).join(' ')}
      style={style}
      remarkPlugins={[remarkGfm]}
      components={components}
      skipHtml
    >
      {children}
    </SafeMarkdown>
  );
}
