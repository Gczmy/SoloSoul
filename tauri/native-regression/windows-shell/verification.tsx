// This layout fixture cannot authenticate, unlock a Vault or reveal critical fields.
const unavailable = async () => false;
const noop = () => {};
export const useWorkspacePasswordGuard = () => ({
  showPwDialog: false,
  bioAvailable: { available: false },
  passwordHint: null,
  passwordVerify: async () => ({ ok: false, method: 'password' as const }),
  handlePwDialogClose: noop,
  handlePwDialogVerify: unavailable,
  handlePwDialogPinSuccess: noop,
  handleBiometricUnlock: unavailable,
});
export const PasswordVerificationDialog = () => null;
