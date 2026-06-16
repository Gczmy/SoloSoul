//! LLM chat screen with streaming output.

use std::sync::mpsc::Receiver;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

/// The state of an active LLM chat session.
pub struct LlmChatState {
    /// Messages displayed in the chat area (newest last).
    pub messages: Vec<ChatLine>,
    /// Current input buffer.
    pub input: String,
    /// Pending streaming response (being built up).
    pub pending_response: String,
    /// Whether a request is in flight.
    pub is_streaming: bool,
    /// Receiver for stream chunks from background thread.
    pub stream_rx: Option<Receiver<StreamChunk>>,
    /// Scroll offset (0 = latest message at bottom).
    pub scroll: usize,
}

/// A line in the chat display.
#[derive(Clone)]
pub enum ChatLine {
    User(String),
    Assistant(String),
    System(String),
    Error(String),
}

/// A chunk received from the streaming thread.
#[derive(Clone)]
pub enum StreamChunk {
    Text(String),
    Done,
    Error(String),
}

impl LlmChatState {
    pub fn new() -> Self {
        Self {
            messages: vec![ChatLine::System(
                "LLM 聊天已就绪。输入您的问题，按 Enter 发送。输入 /back 返回。".into(),
            )],
            input: String::new(),
            pending_response: String::new(),
            is_streaming: false,
            stream_rx: None,
            scroll: 0,
        }
    }

    /// Poll for new stream chunks and update pending response.
    pub fn poll_stream(&mut self) -> bool {
        if let Some(ref rx) = self.stream_rx {
            loop {
                match rx.try_recv() {
                    Ok(StreamChunk::Text(text)) => {
                        self.pending_response.push_str(&text);
                    }
                    Ok(StreamChunk::Done) => {
                        let resp = std::mem::take(&mut self.pending_response);
                        self.messages.push(ChatLine::Assistant(resp));
                        self.is_streaming = false;
                        self.stream_rx = None;
                        return true;
                    }
                    Ok(StreamChunk::Error(msg)) => {
                        self.messages.push(ChatLine::Error(msg));
                        self.is_streaming = false;
                        self.stream_rx = None;
                        return true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.is_streaming = false;
                        self.stream_rx = None;
                        return true;
                    }
                }
            }
        }
        false
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    chat_state: &LlmChatState,
) {
    let layout = Layout::default()
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    // Chat messages area
    render_messages(frame, layout[0], chat_state);

    // Input area
    render_input(frame, layout[1], chat_state);
}

fn render_messages(frame: &mut Frame, area: Rect, state: &LlmChatState) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &state.messages {
        match msg {
            ChatLine::User(text) => {
                lines.push(Line::from(vec![
                    Span::styled("你: ", Style::default().fg(Color::Cyan).bold()),
                    Span::styled(text, Style::default()),
                ]));
            }
            ChatLine::Assistant(text) => {
                for t in text.lines() {
                    lines.push(Line::from(vec![Span::styled(
                        format!("AI: {}", t),
                        Style::default().fg(Color::Green),
                    )]));
                }
            }
            ChatLine::System(text) => {
                lines.push(Line::from(vec![Span::styled(
                    text,
                    Style::default().fg(Color::DarkGray),
                )]));
            }
            ChatLine::Error(text) => {
                lines.push(Line::from(vec![Span::styled(
                    format!("错误: {}", text),
                    Style::default().fg(Color::Red),
                )]));
            }
        }
    }

    // Show pending streaming response
    if state.is_streaming && !state.pending_response.is_empty() {
        for t in state.pending_response.lines() {
            lines.push(Line::from(vec![Span::styled(
                format!("AI: {}", t),
                Style::default().fg(Color::Green),
            )]));
        }
    } else if state.is_streaming {
        lines.push(Line::from(vec![Span::styled(
            "AI: ⏳ 思考中...",
            Style::default().fg(Color::Yellow),
        )]));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("LLM 对话"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_input(
    frame: &mut Frame,
    area: Rect,
    state: &LlmChatState,
) {
    let status = if state.is_streaming {
        "⏳ 等待响应中..."
    } else {
        "输入消息，Enter 发送，Esc 返回"
    };

    let display = if state.input.is_empty() {
        String::new()
    } else {
        state.input.clone()
    };

    let mut spans = vec![Span::styled("> ", Style::default().fg(Color::Cyan))];
    spans.push(Span::styled(display, Style::default()));

    let paragraph = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::ALL).title(status));
    frame.render_widget(paragraph, area);
}
