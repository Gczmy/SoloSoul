import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import remarkGfm from 'remark-gfm';
import { GuideCodeBlock } from './GuideCodeBlock';
import { GuideTip } from './GuideTip';
import { GuideStepper } from './GuideStepper';
import { GuideImage } from './GuideImage';
import { GuideTable } from './GuideTable';
import { GuideCards, type GuideCardItem } from './GuideCards';

interface Segment {
  type: 'markdown' | 'stepper' | 'tip' | 'info' | 'warning' | 'cards';
  content: string;
  title?: string;
  cards?: GuideCardItem[];
}

/** 解析形如 `- [标题](链接) — 描述` 的卡片列表 */
function parseCardItems(content: string): GuideCardItem[] {
  const items: GuideCardItem[] = [];
  const lines = content.split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('- ')) continue;
    // 匹配 - [标题](href) — 描述  或 - [标题](href): 描述
    const match = trimmed.match(/^- \[([^\]]+)\]\(([^)]+)\)\s*[—:-]\s*(.+)$/);
    if (match) {
      items.push({ title: match[1], href: match[2], desc: match[3] });
    }
  }
  return items;
}

/** 解析 Markdown 中的 HTML 注释自定义容器 */
function parseSegments(markdown: string): Segment[] {
  const regex = /<!--(\w+)(?:\s+(.+?))?-->([\s\S]*?)<!--\/\1-->/g;
  const segments: Segment[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(markdown)) !== null) {
    if (match.index > lastIndex) {
      segments.push({ type: 'markdown', content: markdown.slice(lastIndex, match.index) });
    }
    const tag = match[1].toLowerCase();
    const title = match[2]?.trim();
    const content = match[3].trim();
    if (tag === 'stepper') {
      segments.push({ type: 'stepper', content, title });
    } else if (tag === 'tip') {
      segments.push({ type: 'tip', content });
    } else if (tag === 'info') {
      segments.push({ type: 'info', content });
    } else if (tag === 'warning') {
      segments.push({ type: 'warning', content });
    } else if (tag === 'cards') {
      segments.push({ type: 'cards', content, cards: parseCardItems(content) });
    }
    lastIndex = regex.lastIndex;
  }

  if (lastIndex < markdown.length) {
    segments.push({ type: 'markdown', content: markdown.slice(lastIndex) });
  }

  return segments;
}

function createMarkdownComponents(onLinkClick?: (href: string) => void) {
  return {
    code: GuideCodeBlock,
    img: GuideImage,
    table: GuideTable,
    a: ({ href, children }: { href?: string; children?: React.ReactNode }) => {
      if (!href) return <span>{children}</span>;
      // 内部 Markdown 链接（如 templates.md）由调用方处理为应用内导航
      if (href.endsWith('.md')) {
        return (
          <button
            onClick={() => onLinkClick?.(href)}
            style={{
              background: 'none',
              border: 'none',
              padding: 0,
              color: 'var(--accent-primary)',
              textDecoration: 'underline',
              cursor: 'pointer',
              fontSize: 'inherit',
              lineHeight: 'inherit',
            }}
          >
            {children}
          </button>
        );
      }
      return (
        <a
          href={href}
          target="_blank"
          rel="noreferrer"
          style={{ color: 'var(--accent-primary)', textDecoration: 'underline' }}
        >
          {children}
        </a>
      );
    },
    thead: ({ children }: { children?: React.ReactNode }) => (
      <thead style={{ background: 'var(--bg-toolbar)', borderBottom: '2px solid var(--border-subtle)' }}>
        {children}
      </thead>
    ),
    th: ({ children }: { children?: React.ReactNode }) => (
      <th
        style={{
          padding: '10px 14px',
          textAlign: 'left',
          fontSize: 13,
          fontWeight: 600,
          color: 'var(--text-secondary)',
          whiteSpace: 'nowrap',
        }}
      >
        {children}
      </th>
    ),
    td: ({ children }: { children?: React.ReactNode }) => (
      <td
        style={{
          padding: '10px 14px',
          borderBottom: '1px solid var(--border-subtle)',
          fontSize: 14,
          color: 'var(--text-primary)',
        }}
      >
        {children}
      </td>
    ),
    tr: ({ children }: { children?: React.ReactNode }) => (
      <tr style={{ transition: 'background 0.15s ease' }}>{children}</tr>
    ),
    tbody: ({ children }: { children?: React.ReactNode }) => <tbody>{children}</tbody>,
    ol: ({ children }: { children?: React.ReactNode }) => (
      <ol style={{ paddingLeft: 24, margin: '8px 0', listStylePosition: 'outside' }}>{children}</ol>
    ),
    ul: ({ children }: { children?: React.ReactNode }) => (
      <ul style={{ paddingLeft: 24, margin: '8px 0', listStylePosition: 'outside' }}>{children}</ul>
    ),
    li: ({ children }: { children?: React.ReactNode }) => (
      <li style={{ margin: '4px 0', paddingLeft: 4 }}>{children}</li>
    ),
  };
}

interface GuideRendererProps {
  content: string;
  onLinkClick?: (href: string) => void;
}

export function GuideRenderer({ content, onLinkClick }: GuideRendererProps) {
  const segments = parseSegments(content);
  const markdownComponents = createMarkdownComponents(onLinkClick);

  return (
    <div style={{ lineHeight: 1.7, color: 'var(--text-primary)', fontSize: 15 }}>
      {segments.map((seg, i) => {
        if (seg.type === 'markdown') {
          return (
            <ReactMarkdown
              key={i}
              components={markdownComponents}
              remarkPlugins={[remarkGfm]}
              rehypePlugins={[rehypeHighlight]}
            >
              {seg.content}
            </ReactMarkdown>
          );
        }

        if (seg.type === 'stepper') {
          return (
            <GuideStepper key={i} title={seg.title}>
              <ReactMarkdown components={markdownComponents} remarkPlugins={[remarkGfm]}
              rehypePlugins={[rehypeHighlight]}>
                {seg.content}
              </ReactMarkdown>
            </GuideStepper>
          );
        }

        if (seg.type === 'tip') {
          return (
            <GuideTip key={i} type="tip">
              <ReactMarkdown components={markdownComponents} remarkPlugins={[remarkGfm]}
              rehypePlugins={[rehypeHighlight]}>
                {seg.content}
              </ReactMarkdown>
            </GuideTip>
          );
        }

        if (seg.type === 'info') {
          return (
            <GuideTip key={i} type="info">
              <ReactMarkdown components={markdownComponents} remarkPlugins={[remarkGfm]}
              rehypePlugins={[rehypeHighlight]}>
                {seg.content}
              </ReactMarkdown>
            </GuideTip>
          );
        }

        if (seg.type === 'warning') {
          return (
            <GuideTip key={i} type="warning">
              <ReactMarkdown components={markdownComponents} remarkPlugins={[remarkGfm]}
              rehypePlugins={[rehypeHighlight]}>
                {seg.content}
              </ReactMarkdown>
            </GuideTip>
          );
        }

        if (seg.type === 'cards') {
          return <GuideCards key={i} items={seg.cards || []} onLinkClick={onLinkClick} />;
        }

        return null;
      })}
    </div>
  );
}
