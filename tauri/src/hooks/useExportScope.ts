import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AttachmentInfo } from '@/types/exportImport';

interface UseExportScopeOptions {
  accountId: string;
  includeAttachments: boolean;
}

interface UseExportScopeReturn {
  selectedPageIds: Set<string>;
  selectedObjectIds: Set<string>;
  expandedPages: Set<string>;
  selectedAttachmentIds: Set<string>;
  objectAttachments: Map<string, AttachmentInfo[]>;
  expandedObjects: Set<string>;
  togglePage: (sectionType: string, objectIds: string[]) => void;
  toggleObject: (id: string, sectionType: string, allIdsInGroup: string[]) => void;
  toggleObjectExpanded: (objectId: string) => void;
  toggleAttachment: (
    attId: string,
    objectId: string,
    sectionType: string,
    allIdsInGroup: string[],
  ) => void;
  toggleExpandedPage: (sectionType: string) => void;
  totalSelected: number;
  /** 批量加载已选对象的附件（当 includeAttachments 从 false 切为 true 时触发） */
  loadSelectedAttachments: () => void;
}

export function useExportScope({
  accountId,
  includeAttachments,
}: UseExportScopeOptions): UseExportScopeReturn {
  const [selectedPageIds, setSelectedPageIds] = useState<Set<string>>(new Set());
  const [selectedObjectIds, setSelectedObjectIds] = useState<Set<string>>(new Set());
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());
  const [selectedAttachmentIds, setSelectedAttachmentIds] = useState<Set<string>>(new Set());
  const [objectAttachments, setObjectAttachments] = useState<Map<string, AttachmentInfo[]>>(
    new Map(),
  );
  const [expandedObjects, setExpandedObjects] = useState<Set<string>>(new Set());

  const toggleExpandedPage = useCallback((sectionType: string) => {
    setExpandedPages((prev) => {
      const next = new Set(prev);
      if (next.has(sectionType)) next.delete(sectionType);
      else next.add(sectionType);
      return next;
    });
  }, []);

  const togglePage = useCallback(
    (sectionType: string, objectIds: string[]) => {
      setSelectedPageIds((prev) => {
        const next = new Set(prev);
        const isAdding = !next.has(sectionType);
        if (isAdding) next.add(sectionType);
        else next.delete(sectionType);

        setSelectedObjectIds((oPrev) => {
          const oNext = new Set(oPrev);
          for (const id of objectIds) {
            if (isAdding) oNext.add(id);
            else oNext.delete(id);
          }
          return oNext;
        });

        setSelectedAttachmentIds((attPrev) => {
          const attNext = new Set(attPrev);
          for (const id of objectIds) {
            const atts = objectAttachments.get(id) || [];
            for (const att of atts) {
              if (isAdding) attNext.add(att.id);
              else attNext.delete(att.id);
            }
          }
          return attNext;
        });

        if (isAdding && includeAttachments) {
          const unloadedIds = objectIds.filter((id) => !objectAttachments.has(id));
          if (unloadedIds.length > 0) {
            Promise.all(
              unloadedIds.map((id) =>
                invoke<AttachmentInfo[]>('export_get_attachments', { accountId, objectId: id })
                  .then((atts) => ({ id, atts }))
                  .catch(() => ({ id, atts: [] as AttachmentInfo[] })),
              ),
            ).then((results) => {
              setObjectAttachments((prev) => {
                const n = new Map(prev);
                for (const { id, atts } of results) n.set(id, atts);
                return n;
              });
              setSelectedAttachmentIds((prev) => {
                const n = new Set(prev);
                for (const { atts } of results) for (const att of atts) n.add(att.id);
                return n;
              });
            });
          }
        }
        return next;
      });
    },
    [accountId, includeAttachments, objectAttachments],
  );

  const toggleObject = useCallback(
    (id: string, sectionType: string, allIdsInGroup: string[]) => {
      setSelectedObjectIds((prev) => {
        const next = new Set(prev);
        const isAdding = !next.has(id);
        if (isAdding) next.add(id);
        else next.delete(id);

        setSelectedPageIds((pPrev) => {
          const pNext = new Set(pPrev);
          const allSelectedNow = allIdsInGroup.every((oid) => next.has(oid));
          if (allSelectedNow) pNext.add(sectionType);
          else pNext.delete(sectionType);
          return pNext;
        });

        setSelectedAttachmentIds((attPrev) => {
          const attNext = new Set(attPrev);
          const atts = objectAttachments.get(id) || [];
          for (const att of atts) {
            if (isAdding) attNext.add(att.id);
            else attNext.delete(att.id);
          }
          return attNext;
        });

        if (isAdding && includeAttachments && !objectAttachments.has(id)) {
          invoke<AttachmentInfo[]>('export_get_attachments', { accountId, objectId: id })
            .then((atts) => {
              setObjectAttachments((prev) => {
                const n = new Map(prev);
                n.set(id, atts);
                return n;
              });
              setSelectedAttachmentIds((prev) => {
                const n = new Set(prev);
                for (const att of atts) n.add(att.id);
                return n;
              });
            })
            .catch((err) => console.warn('[useExportScope] Load attachments failed:', err));
        }
        return next;
      });
    },
    [accountId, includeAttachments, objectAttachments],
  );

  const toggleObjectExpanded = useCallback(
    (objectId: string) => {
      setExpandedObjects((prev) => {
        const next = new Set(prev);
        if (next.has(objectId)) {
          next.delete(objectId);
          return next;
        }
        next.add(objectId);
        if (!objectAttachments.has(objectId)) {
          invoke<AttachmentInfo[]>('export_get_attachments', { accountId, objectId })
            .then((atts) => {
              setObjectAttachments((p) => {
                const n = new Map(p);
                n.set(objectId, atts);
                return n;
              });
            })
            .catch((err) =>
              console.warn('[useExportScope] Load attachment for expanded object failed:', err),
            );
        }
        return next;
      });
    },
    [accountId, objectAttachments],
  );

  const toggleAttachment = useCallback(
    (attId: string, objectId: string, sectionType: string, allIdsInGroup: string[]) => {
      setSelectedAttachmentIds((prev) => {
        const next = new Set(prev);
        const isAdding = !next.has(attId);
        if (isAdding) next.add(attId);
        else next.delete(attId);
        return next;
      });

      setSelectedObjectIds((prev) => {
        const next = new Set(prev);
        if (!next.has(objectId)) {
          next.add(objectId);
          setSelectedPageIds((pagePrev) => {
            const pageNext = new Set(pagePrev);
            const allSelectedNow = allIdsInGroup.every((oid) => next.has(oid));
            if (allSelectedNow) pageNext.add(sectionType);
            return pageNext;
          });
        }
        return next;
      });
    },
    [],
  );

  /** 批量加载已选对象的附件并将它们加入 selectedAttachmentIds */
  const loadSelectedAttachments = useCallback(() => {
    const ids = Array.from(selectedObjectIds);
    const unloadedIds = ids.filter((id) => !objectAttachments.has(id));
    if (unloadedIds.length === 0) return;
    Promise.all(
      unloadedIds.map((id) =>
        invoke<AttachmentInfo[]>('export_get_attachments', { accountId, objectId: id })
          .then((atts) => ({ id, atts }))
          .catch(() => ({ id, atts: [] as AttachmentInfo[] })),
      ),
    ).then((results) => {
      setObjectAttachments((prev) => {
        const n = new Map(prev);
        for (const { id, atts } of results) n.set(id, atts);
        return n;
      });
      setSelectedAttachmentIds((prev) => {
        const n = new Set(prev);
        for (const { atts } of results) for (const att of atts) n.add(att.id);
        return n;
      });
    });
  }, [accountId, selectedObjectIds, objectAttachments]);

  const totalSelected = selectedObjectIds.size;

  return {
    selectedPageIds,
    selectedObjectIds,
    expandedPages,
    selectedAttachmentIds,
    objectAttachments,
    expandedObjects,
    togglePage,
    toggleObject,
    toggleObjectExpanded,
    toggleAttachment,
    toggleExpandedPage,
    totalSelected,
    loadSelectedAttachments,
  };
}
