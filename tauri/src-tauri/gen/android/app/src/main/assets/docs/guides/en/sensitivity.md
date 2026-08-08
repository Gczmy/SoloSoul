# Sensitivity and Privacy

SoloSoul uses a four-level sensitivity system to provide fine-grained data protection for every field.

## Sensitivity Levels

| Level | Description | Display Behavior |
|-------|-------------|------------------|
| `public` | Publicly visible | Plain text |
| `internal` | Internal use only | Plain text with label |
| `sensitive` | Handle with care | Blurred by default; click to reveal |
| `critical` | Highly sensitive | Blurred by default; requires password to reveal |

> Levels are shown as in-app badges: `public` green, `internal` blue, `sensitive` amber, `critical` red (all with a colored border).

## Field-level Protection

Each field in an object template has a preset sensitivity level. For example:

- Passport number → `critical`
- Full name → `public`
- Bank account number → `sensitive`

## Viewing Sensitive Fields

- **`sensitive`**: click the **Reveal** button to view it temporarily
- **`critical`**: clicking opens a master-password verification dialog. Enter your password to view; the action is written to the audit log

<!--TIP-->
Every view of a `critical` field is recorded in the operation log so access history can be traced.
<!--/TIP-->

## Adjusting Sensitivity

A field's sensitivity level is defined in the **object template editor** and can be changed at any time:

1. Go to **Templates** → edit the target template
2. Find the target field's row
3. Pick a new level from the **Sensitivity** dropdown
4. Save the template

After the change, all objects created from that template use the new protection level.

## Privacy Boundary with AI

- Only `public`-level field information is injected into AI system prompts
- `sensitive` / `critical` data is **never** sent to AI
- You can disable system prompt injection in **LLM Config**

## Related Docs

<!--CARDS-->
- [Security Settings](security.md) — Password and vault
- [Biometrics](biometric.md) — Fingerprint and face unlock
- [AI Chat](ai_chat.md) — AI privacy boundaries
<!--/CARDS-->
