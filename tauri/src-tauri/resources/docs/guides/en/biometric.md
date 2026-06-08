# Biometrics

SoloSoul supports using device biometrics (Touch ID / Face ID) for quick vault unlocking and sensitive operation verification.

## Supported Platforms

| Platform | Supported Types |
|----------|-----------------|
| macOS | Touch ID |
| iOS | Face ID / Touch ID |
| Windows | Windows Hello (planned) |
| Android | Fingerprint (planned) |

## Enabling Biometrics

1. Go to **Settings → Security Settings**
2. Check the status in the **Biometric Unlock** section
3. If it shows **Available**, click **Enable**
4. Enter your master password to verify identity
5. Follow the system prompt to complete biometric enrollment

<!--STEPPER Enable Touch ID-->
1. Go to **Settings → Security Settings**
2. In the biometrics section, click **Enable Touch ID**
3. Enter your master password
4. Place your finger on the Touch ID sensor
5. Success — return to settings
<!--/STEPPER-->

## Use Cases

Once enabled, biometrics can be used for:

- Unlocking the vault (instead of typing the password)
- Viewing `critical`-level sensitive fields
- Performing sensitivity downgrade operations

## Disabling Biometrics

1. Go to **Settings → Security Settings**
2. Click **Disable Touch ID**
3. Enter your master password to confirm

<!--WARNING-->
After changing your master password, biometric credentials automatically become invalid and must be re-enabled.
<!--/WARNING-->

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Biometrics unavailable | Check system preferences to ensure fingerprint/face is enrolled |
| Too many failed attempts | The system temporarily disables biometrics. Use your password to unlock |
| Password changed | Re-enable biometrics in Security Settings |
