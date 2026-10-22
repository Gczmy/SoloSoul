/** Payload-free invalidation: subscribers never receive decrypted fields or IPC arguments. */
const listeners = new Set<() => void>();
const writes = new Set([
  'object_create',
  'object_update',
  'object_set_locked',
  'object_delete',
  'page_delete',
  'trash_restore',
  'trash_restore_batch',
  'import_execute_advanced',
  'import_operation_resume',
  'backup_restore',
  'sync_with_device',
  'sync_resolve_conflict',
]);

export function onVaultDataChange(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function notifyVaultDataChange() {
  for (const listener of listeners) {
    // A UI refresh must never turn a successful write into an apparent failure.
    try {
      listener();
    } catch {
      /* The next explicit refresh can retry. */
    }
  }
}

export function notifyVaultCommand(
  cmd: string,
  args?: Record<string, unknown>,
  requestIsCurrent?: () => boolean,
) {
  if (
    (writes.has(cmd) || (cmd === 'object_sync_with_template' && args?.dryRun === false)) &&
    (!requestIsCurrent || requestIsCurrent())
  ) {
    notifyVaultDataChange();
  }
}
