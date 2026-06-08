import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import { GuideCodeBlock } from './GuideCodeBlock';
import { GuideTip } from './GuideTip';
import { GuideStepper } from './GuideStepper';
import { GuideImage } from './GuideImage';
import { GuideTable } from './GuideTable';

interface Segment {
  type: 'markdown' | 'stepper' | 'tip' | 'warning';
  content: string;
  title?: string;
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
    } else if (tag === 'warning') {
      segments.push({ type: 'warning', content });
    }
    lastIndex = regex.lastIndex;
  }

  if (lastIndex < markdown.length) {
    segments.push({ type: 'markdown', content: markdown.slice(lastIndex) });
  }

  return segments;
}

const markdownComponents = {
  code: GuideCodeBlock as any,
  img: GuideImage as any,
  table: GuideTable as any,
  thead: ({ children }: any) => (
    <thead style={{ background: 'var(--bg-toolbar)', borderBottom: '2px solid var(--border-subtle)' }}>
      {children}
    </thead>
  ),
  th: ({ children }: any) => (
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
  td: ({ children }: any) => (
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
  tr: ({ children }: any) => (
    <tr style={{ transition: 'background 0.15s ease' }}>{children}</tr>
  ),
  tbody: ({ children }: any) => <tbody>{children}</tbody>,
};

interface GuideRendererProps {
  content: string;
}

export function GuideRenderer({ content }: GuideRendererProps) {
  const segments = parseSegments(content);

  return (
    <div style={{ lineHeight: 1.7, color: 'var(--text-primary)', fontSize: 15 }}>
      {segments.map((seg, i) => {
        if (seg.type === 'markdown') {
          return (
            <ReactMarkdown
              key={i}
              components={markdownComponents}
              rehypePlugins={[rehypeHighlight]}
            >
              {seg.content}
            </ReactMarkdown>
          );
        }

        if (seg.type === 'stepper') {
          return (
            <GuideStepper key={i} title={seg.title}>
              <ReactMarkdown components={markdownComponents} rehypePlugins={[rehypeHighlight]}>
                {seg.content}
              </ReactMarkdown>
            </GuideStepper>
          );
        }

        if (seg.type === 'tip') {
          return (
            <GuideTip key={i} type="tip">
              <ReactMarkdown components={markdownComponents} rehypePlugins={[rehypeHighlight]}>
                {seg.content}
              </ReactMarkdown>
            </GuideTip>
          );
        }

        if (seg.type === 'warning') {
          return (
            <GuideTip key={i} type="warning">
              <ReactMarkdown components={markdownComponents} rehypePlugins={[rehypeHighlight]}>
                {seg.content}
              </ReactMarkdown>
            </GuideTip>
          );
        }

        return null;
      })}
    </div>
  );
}
