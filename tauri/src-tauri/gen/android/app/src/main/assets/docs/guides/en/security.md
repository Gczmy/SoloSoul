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

## Auto-lock

SoloSoul automatically locks the vault when:

- The app enters the background for longer than the set duration
- System screen lock is detected
- You manually click **Lock Vault** in the sidebar

After locking, all sensitive state is cleared. You must re-enter your password or use biometrics to unlock.

## Related Docs

<!--CARDS-->
- [Sensitivity & Privacy](sensitivity.md) — Field-level protection
- [Biometrics](biometric.md) — Convenient unlock methods
- [Backup & Restore](backup_restore.md) — Protect your data
<!--/CARDS-->

