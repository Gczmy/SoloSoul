# LLM Config & Statistics

LLM Config manages AI providers and feature switches. The Statistics page displays your AI usage data.

## Configuring Providers

### Built-in Providers

SoloSoul includes the following providers:

- OpenAI (GPT series)
- Anthropic (Claude series)
- DeepSeek

Setup steps:

1. Go to **Settings → AI & LLM Config**
2. Select a built-in provider
3. Enter your API key
4. Click **Test Connection** to verify
5. Set it as the active provider

### Custom Providers

To use another OpenAI API-compatible service:

1. Click **Add Custom Provider**
2. Enter name, base URL, and model name
3. Select API type (OpenAI / Anthropic)
4. Enter API key
5. Save and test connection

<!--TIP-->
For local models (e.g., Ollama), the base URL is typically `http://localhost:11434/v1`.
<!--/TIP-->

## AI Feature Switches

You can individually control AI features in the LLM Config page:

| Feature | Description |
|---------|-------------|
| AI Chat | Chat with the AI assistant |
| Smart Fill | Auto-complete object fields (in development) |
| Command Gen | Natural language command generation (in development) |
| Natural Language Search | Search by natural language description (in development) |

## System Prompt

The system prompt switch controls whether software info is injected into AI context:

- **On**: AI knows your public data and app state for more accurate answers
- **Off**: Only user input is sent for maximum privacy

## Usage Statistics

Click the **Statistics** icon in the top-right of the LLM Config page to view:

- Total conversations and token usage
- Usage share by model
- Prompt / Completion token distribution
- Daily usage trend over the last 14 days
- Online status detection

<!--TIP-->
Token statistics are obtained by parsing real return data from AI providers. Both OpenAI and Anthropic support precise usage reporting.
<!--/TIP-->

## Related Docs

<!--CARDS-->
- [AI Chat](ai_chat.md) — Use the AI assistant
- [Plugins](plugins.md) — Plugins and LLM
- [Privacy Policy](PRIVACY_POLICY.md) — Data privacy statement
<!--/CARDS-->

