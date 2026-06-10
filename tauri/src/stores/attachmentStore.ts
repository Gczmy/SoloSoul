import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface AttachmentMeta {
  id: string;
  objectId: string;
  fileName: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
}

interface AttachmentState {
  attachments: Record<string, AttachmentMeta[]>; // objectId → attachments
  isLoading: boolean;
  error: string | null;

  loadAttachments: (objectId: string) => Promise<void>;
  addAttachment: (objectId: string, filePath: string) => Promise<void>;
  removeAttachment: (objectId: string, attachmentId: string) => Promise<void>;
  decryptAttachment: (attachmentId: string, outputPath: string) => Promise<void>;
}

let counter = 0;
function genId(): string {
  return `att_${Date.now()}_${++counter}`;
}

export const useAttachmentStore = create<AttachmentState>((set, _get) => ({
  attachments: {},
  isLoading: false,
  error: null,

  loadAttachments: async (objectId) => {
    set({ isLoading: true, error: null });
    try {
      const list = await invoke<AttachmentMeta[]>('attachment_list', { objectId });
      set((s) => ({ attachments: { ...s.attachments, [objectId]: list }, isLoading: false }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  addAttachment: async (objectId, filePath) => {
    set({ isLoading: true, error: null });
    try {
      const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file';
      const id = genId();
      // Encrypt file content
      const encryptedPath = `${filePath}.solo`;
      await invoke('encrypt_file', { srcPath: filePath, dstPath: encryptedPath });

      const meta: AttachmentMeta = {
        id,
        objectId,
        fileName,
        mimeType: guessMime(fileName),
        sizeBytes: 0, // Will be set by backend
        createdAt: new Date().toISOString(),
      };

      await invoke('attachment_save', { objectId, meta });

      set((s) => ({
        attachments: {
          ...s.attachments,
          [objectId]: [...(s.attachments[objectId] || []), meta],
        },
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  removeAttachment: async (objectId, attachmentId) => {
    set({ isLoading: true });
    try {
      await invoke('attachment_delete', { objectId, attachmentId });
      set((s) => ({
        attachments: {
          ...s.attachments,
          [objectId]: (s.attachments[objectId] || []).filter((a) => a.id !== attachmentId),
        },
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  decryptAttachment: async (_attachmentId, outputPath) => {
    // Decryption is handled by the backend's decrypt_file command
    await invoke('decrypt_file', { srcPath: '', dstPath: outputPath }).catch(() => {});
  },
}));

function guessMime(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    pdf: 'application/pdf',
    png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg',
    gif: 'image/gif', webp: 'image/webp',
    txt: 'text/plain', md: 'text/markdown',
    json: 'application/json', xml: 'application/xml',
  };
  return map[ext] || 'application/octet-stream';
}
