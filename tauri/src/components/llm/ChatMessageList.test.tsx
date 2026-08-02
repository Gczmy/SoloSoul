import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from '@testing-library/react';
import { ChatMessageList } from './ChatMessageList';
import type { ChatMsg } from '@/types/llmChat';

// P116: 统计每条 content 触发 SafeMarkdown 渲染的次数，用于验证 memo
// 生效——流式期间仅最后一条 assistant 消息重新渲染。
const { renderCounts } = vi.hoisted(() => ({
  renderCounts: new Map<string, number>(),
}));

vi.mock('@/components/ui/SafeMarkdown', () => ({
  SafeMarkdown: ({ children }: { children: string }) => {
    renderCounts.set(children, (renderCounts.get(children) ?? 0) + 1);
    return <div data-testid="markdown">{children}</div>;
  },
}));

function makeMsg(id: string, role: 'user' | 'assistant', content: string): ChatMsg {
  return { id, role, content, createdAt: '2026-08-02T08:00:00.000Z' };
}

const baseProps = {
  isSending: false,
  copiedIndex: null,
  onCopy: () => {},
  errorPrefix: '[ERROR]',
  activeProviderName: 'test-provider',
  scrollContainerRef: { current: null },
  chatEndRef: { current: null },
};

describe('ChatMessageList', () => {
  beforeEach(() => {
    renderCounts.clear();
  });

  it('renders user and assistant message content', () => {
    const messages = [makeMsg('u1', 'user', 'hello'), makeMsg('a1', 'assistant', 'world')];
    const { getByText } = render(<ChatMessageList {...baseProps} messages={messages} />);
    expect(getByText('hello')).toBeInTheDocument();
    expect(getByText('world')).toBeInTheDocument();
  });

  it('renders error-prefixed assistant content as plain text', () => {
    const messages = [makeMsg('a1', 'assistant', '[ERROR]: boom')];
    const { getByText } = render(<ChatMessageList {...baseProps} messages={messages} />);
    expect(getByText('[ERROR]: boom')).toBeInTheDocument();
  });

  it('shows typing indicator while sending', () => {
    const messages = [makeMsg('u1', 'user', 'hello')];
    const { container } = render(
      <ChatMessageList {...baseProps} messages={messages} isSending={true} />,
    );
    expect(container.querySelector('.typing-animation')).not.toBeNull();
  });

  it('P116: streaming update re-renders only the last assistant message', () => {
    const firstMsg = makeMsg('a1', 'assistant', 'first reply');
    const streamingMsg = makeMsg('a2', 'assistant', 'hi');
    const messages = [firstMsg, streamingMsg];

    const { rerender } = render(<ChatMessageList {...baseProps} messages={messages} />);
    // 初始渲染：两条 assistant 消息各渲染一次
    expect(renderCounts.get('first reply')).toBe(1);
    expect(renderCounts.get('hi')).toBe(1);

    // 模拟流式更新：仅最后一条 assistant 消息 content 变化，
    // 前面的消息对象保持引用稳定。
    const updatedMessages = [firstMsg, { ...streamingMsg, content: 'hi there, how are you?' }];
    rerender(<ChatMessageList {...baseProps} messages={updatedMessages} />);

    // 关键断言：前一条消息未重新渲染（memo 命中），新内容渲染一次
    expect(renderCounts.get('first reply')).toBe(1);
    expect(renderCounts.get('hi there, how are you?')).toBe(1);
  });
});
