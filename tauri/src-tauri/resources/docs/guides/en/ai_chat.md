# AI Chat

SoloSoul's AI Chat lets you communicate with an AI assistant directly within the app while maintaining full control over your data.

## Enabling AI Chat

1. Open **Settings → AI & LLM Config**
2. Read the risk notice and check **I understand the risks**
3. Configure an AI provider:
   - Choose a built-in provider (OpenAI, Anthropic, DeepSeek, etc.)
   - Or add a custom provider
   - Enter your API key
4. Turn on the **AI Chat** feature switch

<!--STEPPER Configure AI Chat-->
1. Go to **Settings → AI & LLM Config**
2. Read and acknowledge the risk notice
3. Select a provider and enter your API key
4. Click **Test Connection** to verify availability
5. Turn on the **AI Chat** switch
<!--/STEPPER-->

## How to Use

1. Click the **AI Chat** icon in the sidebar
2. Type your question in the input box and press Enter
3. The AI assistant responds based on your publicly shared info and software docs

## System Prompt

The system prompt is context information automatically injected into every conversation, including:

- Basic software information
- Your objects marked as `public` level
- Usage statistics (conversation count, token usage, etc.)

You can disable system prompt injection in LLM Config.

<!--TIP-->
The system prompt only includes info you actively share. Sensitive / restricted / critical data is never sent to AI.
<!--/TIP-->

## Conversation Management

- **New Conversation**: Click **New Conversation** to start a fresh chat thread
- **Delete**: Click the delete icon in the conversation list. Conversations move to Trash
- **Trash**: Click **Trash** on the chat page to view and restore deleted conversations

## Privacy Notes

- Conversations are streamed via SSE (Server-Sent Events)
- All conversation history is stored locally only
- AI providers may process your data according to their privacy policies

<!--WARNING-->
AI features send data to external LLM providers. Do not share sensitive or restricted data via AI chat.
<!--/WARNING-->
