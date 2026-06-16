import { invoke } from '@tauri-apps/api/core';

export interface SyncConflict {
  table: string;
  id: string;
  local_hlc: {
    wall_time_ms: number;
    counter: number;
    node_id: number[];
  };
  remote_hlc: {
    wall_time_ms: number;
    counter: number;
    node_id: number[];
  };
  winner: string;
}

export interface SyncTableResult {
  table: string;
  examined: number;
  applied: number;
  skipped: number;
}

export interface SyncResult {
  summary: string;
  examined: number;
  applied: number;
  skipped: number;
  conflicts: SyncConflict[];
  per_table: SyncTableResult[];
}

export interface AccountInfo {
  id: string;
  name: string;
  salt?: string;
  verifyHash?: string;
  passwordHint?: string;
  createdAt?: string;
}

export interface ProfileSummary {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  version: number;
}

export interface Profile {
  id: string;
  name: string;
  data: number[];
  createdAt: string;
  updatedAt: string;
  version: number;
}

export type VaultStateStr = 'uninitialized' | 'locked' | 'unlocked';

export const commands = {
  // Auth
  async checkHasAccount(): Promise<boolean> {
    return invoke('check_has_account');
  },
  async bootstrap(
    accountName: string,
    password: string,
    locale: string,
    passwordHint?: string,
  ): Promise<AccountInfo> {
    return invoke('bootstrap', { accountName, password, locale, passwordHint });
  },
  async login(accountId: string, password: string): Promise<void> {
    return invoke('login', { accountId, password });
  },
  async logout(): Promise<void> {
    return invoke('logout');
  },

  // Vault
  async vaultUnlock(accountId: string, password: string): Promise<void> {
    return invoke('unlock', { accountId, password });
  },
  async vaultLock(): Promise<void> {
    return invoke('lock');
  },
  async vaultGetState(): Promise<VaultStateStr> {
    return invoke('get_state');
  },
  async vaultChangePassword(
    accountId: string,
    oldPassword: string,
    newPassword: string,
  ): Promise<void> {
    return invoke('change_password', { accountId, oldPassword, newPassword });
  },
  async vaultDeleteAccount(accountId: string, password: string): Promise<void> {
    return invoke('delete_account', { accountId, password });
  },
  async vaultListAccounts(): Promise<AccountInfo[]> {
    return invoke<AccountInfo[]>('vault_list_accounts');
  },

  // Profile
  async profileSave(accountId: string, name: string, data: number[]): Promise<ProfileSummary> {
    return invoke('profile_save', { payload: { accountId, name, data } });
  },
  async profileLoad(accountId: string): Promise<Profile | null> {
    return invoke('profile_load', { accountId });
  },
  async profileList(): Promise<ProfileSummary[]> {
    return invoke('profile_list');
  },
  async profileDelete(profileId: string): Promise<void> {
    return invoke('profile_delete', { profileId });
  },

  // Crypto
  async encryptBytes(data: number[]): Promise<number[]> {
    return invoke('encrypt_bytes', { data });
  },
  async decryptBytes(data: number[]): Promise<number[]> {
    return invoke('decrypt_bytes', { data });
  },
  async encryptWithKey(key: number[], plaintext: number[]): Promise<number[]> {
    return invoke('encrypt_with_key', { key, plaintext });
  },
  async decryptWithKey(key: number[], ciphertext: number[]): Promise<number[]> {
    return invoke('decrypt_with_key', { key, ciphertext });
  },
  async deriveKey(
    password: string,
    salt: number[],
    memoryKib: number,
    iterations: number,
    parallelism: number,
  ): Promise<number[]> {
    return invoke('derive_key', { password, salt, memoryKib, iterations, parallelism });
  },
  async generateSalt(length: number): Promise<number[]> {
    return invoke('generate_salt', { length });
  },
  async constantTimeCompare(a: number[], b: number[]): Promise<boolean> {
    return invoke('constant_time_compare', { a, b });
  },
  async getVaultStats(): Promise<{
    profileCount: number;
    totalSizeBytes: number;
    lastModified: string | null;
  }> {
    return invoke('get_vault_stats');
  },

  // File System
  async inspectBackup(backupPath: string): Promise<string> {
    return invoke('inspect_backup', { backupPath });
  },

  // Discovery
  async mdnsDiscover(
    timeoutMs: number,
  ): Promise<{ name: string; host: string; port: number; addresses: string[] }[]> {
    return invoke('mdns_discover', { timeoutMs });
  },
  async mdnsAdvertise(deviceName: string, port: number): Promise<void> {
    return invoke('mdns_advertise', { deviceName, port });
  },

  // Sync
  async syncGetStatus(): Promise<{
    isDiscovering: boolean;
    syncEnabled: boolean;
    localFingerprint: string;
    connectedPeers: Array<{
      id: string;
      name: string;
      addr: string;
      fingerprint: string;
      trusted: boolean;
      lastSeen: string;
    }>;
  }> {
    return invoke('sync_get_status');
  },
  async syncEnable(enable: boolean): Promise<void> {
    return invoke('sync_enable', { enable });
  },
  async syncWithDevice(deviceId: string): Promise<SyncResult> {
    return invoke('sync_with_device', { deviceId });
  },
  async syncTrustPeer(peerNodeId: string, trusted: boolean): Promise<void> {
    return invoke('sync_trust_peer', { peerNodeId, trusted });
  },
  async syncForgetPeer(peerNodeId: string): Promise<void> {
    return invoke('sync_forget_peer', { peerNodeId });
  },

  // OCR
  async ocrScanImage(filePath: string, language?: string): Promise<OcrResult> {
    return invoke('ocr_scan_image', { filePath, language });
  },
  async ocrGetSupportedLanguages(): Promise<string[]> {
    return invoke('ocr_get_supported_languages');
  },
  async ocrListAvailableTiers(): Promise<OcrTierInfo[]> {
    return invoke('ocr_list_available_tiers');
  },
  async ocrGetActiveTier(): Promise<string> {
    return invoke('ocr_get_active_tier');
  },
  async ocrSetActiveTier(tier: string): Promise<void> {
    return invoke('ocr_set_active_tier', { tier });
  },
  async ocrGetModelStatus(tier: string): Promise<OcrModelStatus> {
    return invoke('ocr_get_model_status', { tier });
  },
  async ocrInstallBundledModel(tier: string): Promise<void> {
    return invoke('ocr_install_bundled_model', { tier });
  },
  async ocrDownloadModel(tier: string, baseUrl: string): Promise<void> {
    return invoke('ocr_download_model', { tier, baseUrl });
  },
};

export interface OcrBox {
  text: string;
  confidence: number;
  points: [number, number][];
}

export interface OcrResult {
  text: string;
  confidence: number;
  boxes: OcrBox[];
}

export interface OcrTierInfo {
  tier: string;
  name: string;
  description: string;
}

export interface OcrModelStatus {
  tier: string;
  installed: boolean;
  bundled: boolean;
}
