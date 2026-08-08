# Security Settings

Security Settings manage your account's core security features: password, biometrics, and auto-lock.

## Changing Password

1. Go to **Settings → Security Settings**
2. Enter your current password
3. Enter and confirm your new password
4. Click **Change Password**

<!--WARNING-->
After changing your password, all biometric credentials become invalid and must be re-enabled.
<!--/WARNING-->

## Password Hint

The password hint is auxiliary memory information shown on the login screen:

- It should not contain the password itself or overly obvious clues
- You can update the hint when changing your password
- Leave blank to not display on the login screen

## Biometric Unlock

If your device supports biometrics (e.g., Touch ID / Face ID):

1. Go to **Settings → Security Settings**
2. Check availability in the **Biometric Unlock** section
3. Click **Enable Touch ID**
4. Enter your master password for verification
5. Follow the prompt to complete biometric enrollment

<!--TIP-->
Biometric credentials are bound to your master password. After changing the password, biometrics must be set up again.
<!--/TIP-->

## Testing Biometrics

After enabling, you can click **Test Touch ID** to verify the feature works.

## PIN Unlock

If your device supports it, you can set a **6-digit PIN** for quick unlock as an alternative to biometrics:

1. Go to **Settings → Security Settings**
2. In the **PIN** section, click **Set PIN**
3. Enter your master password to verify your identity
4. Set a 6-digit PIN and confirm it again

Once configured, you can unlock with the PIN directly when the vault is locked. You can change or disable the PIN at any time (disabling also requires the master password).

<!--TIP-->
Repeated wrong PIN entries temporarily lock out the PIN to deter brute-force attempts.
<!--/TIP-->

## Auto-lock

SoloSoul automatically locks the vault when:

- No activity for the configured auto-lock duration (1 / 5 / 15 / 30 minutes, or **Never**)
- The app is moved to the background (this switch can be turned off)
- You manually click **Lock Vault** in the sidebar

After locking, all sensitive state is cleared. You must re-enter your password, PIN, or use biometrics to unlock.

## Related Docs

<!--CARDS-->
- [Sensitivity & Privacy](sensitivity.md) — Field-level protection
- [Biometrics](biometric.md) — Convenient unlock methods
- [Backup & Restore](backup_restore.md) — Protect your data
- [Device Sync](device_sync.md) — Secure multi-device sync
<!--/CARDS-->

