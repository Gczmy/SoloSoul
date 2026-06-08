# Sensitivity and Privacy

SoloSoul uses a six-level sensitivity system to provide fine-grained data protection for every field.

## Sensitivity Levels

| Level | Description | Display Behavior |
|-------|-------------|------------------|
| public | Public | Plain text |
| internal | Internal | Plain text with label |
| private | Private | Plain text with label |
| sensitive | Sensitive | Blurred by default; click to reveal |
| restricted | Restricted | Blurred by default; click to reveal |
| critical | Critical | Blurred by default; requires password to reveal |

## Field-level Protection

Each field in an object template has a preset sensitivity level. For example:

- Passport number → `critical`
- Full name → `public`
- Bank account number → `sensitive`

## Viewing Sensitive Fields

- **sensitive / restricted**: Click the blurred area to temporarily reveal
- **critical**: Clicking opens a password verification dialog. Enter your master password to view

<!--TIP-->
Biometric unlock (e.g., Touch ID) can substitute for password entry when viewing critical fields, if enabled in Settings.
<!--/TIP-->

## Adjusting Sensitivity

To change a field's protection level:

1. Go to **Settings → Sensitivity Settings**
2. Find the target field
3. Select a new sensitivity level
4. (When downgrading) Enter your master password and provide a reason
5. Confirm the change

<!--WARNING-->
Downgrading field protection requires password verification and is recorded in the audit log. Changes cannot be reverted without a trace.
<!--/WARNING-->

## Privacy Boundary with AI

- Only `public`-level field information is injected into AI system prompts
- `sensitive` / `restricted` / `critical` data is **never** sent to AI
- You can disable system prompt injection in **LLM Config**
