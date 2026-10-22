import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { ObjectInspectorContext } from '@/components/layout/shellConfigStore';
import type { ObjectInspectorTab } from '@/stores/uiStore';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import type { SnapshotEntry } from '@/types/history';
import { createSessionRequests } from '@/lib/sessionRequests';

type InspectorData =
  | { tab: 'history'; items: SnapshotEntry[] }
  | { tab: 'attachments'; items: AttachmentItem[] };
type InspectorView = {
  key: string;
  context: ObjectInspectorContext;
  data: InspectorData;
};

/** 列表与编辑器并行读取；名称和元数据作为一个完整画面交接，不保存解密正文。 */
export function useObjectInspectorTransition(
  context: ObjectInspectorContext,
  tab: ObjectInspectorTab,
  accountId?: string,
) {
  const [requests] = useState(createSessionRequests);
  const sequence = useRef(0);
  const [attempt, setAttempt] = useState(0);
  const key = JSON.stringify([context.objectId, tab, attempt]);
  const [result, setResult] = useState<{
    key: string;
    version: number;
    data?: InspectorData;
    failed?: boolean;
  }>();
  const [displayed, setDisplayed] = useState<InspectorView>();
  const objectFailed = Boolean(context.failed);
  useLayoutEffect(() => {
    const request = requests.begin('metadata', accountId);
    const version = ++sequence.current;
    setResult(undefined);
    if (!objectFailed) {
      const read: Promise<InspectorData> =
        tab === 'history'
          ? request.invokeTyped('snapshot_list', { objectId: context.objectId }).then((items) => ({
              tab: 'history',
              items: [...items].sort((a, b) => b.timestamp - a.timestamp),
            }))
          : request
              .invoke<AttachmentItem[]>('attachment_list', {
                objectId: context.objectId,
                showDeleted: false,
              })
              .then((items) => ({
                tab: 'attachments',
                items: items
                  .filter((item) => item.objectId === context.objectId && !item.deletedAt)
                  .sort((a, b) => (Date.parse(b.createdAt) || 0) - (Date.parse(a.createdAt) || 0)),
              }));
      void read
        .then((data) => {
          if (request.isCurrent()) setResult({ key, version, data });
        })
        .catch(() => {
          if (request.isCurrent()) setResult({ key, version, failed: true });
        });
    }
    return () => requests.invalidate('metadata');
  }, [accountId, context.objectId, key, objectFailed, requests, tab]);
  useLayoutEffect(() => () => requests.invalidate(), [requests]);

  const currentResult = result?.key === key ? result : undefined;
  const failed = objectFailed || Boolean(context.ready && currentResult?.failed);
  const incoming = useMemo<InspectorView | undefined>(
    () =>
      context.ready && !objectFailed && currentResult?.data
        ? {
            key: `${key}:${currentResult.version}`,
            context: {
              objectId: context.objectId,
              objectName: context.objectName,
              typeId: context.typeId,
              ready: true,
            },
            data: currentResult.data,
          }
        : undefined,
    [
      context.objectId,
      context.objectName,
      context.typeId,
      context.ready,
      objectFailed,
      currentResult,
      key,
    ],
  );
  useLayoutEffect(() => {
    if (failed) setDisplayed(undefined);
    else if (incoming) setDisplayed(incoming);
  }, [failed, incoming]);
  const view = failed
    ? undefined
    : (incoming ?? (displayed?.data.tab === tab ? displayed : undefined));
  const pending = !failed && !incoming;
  const [noticeKey, setNoticeKey] = useState<string>();
  useEffect(() => {
    if (!pending) {
      setNoticeKey(undefined);
      return;
    }
    const timer = setTimeout(() => setNoticeKey(key), 250);
    return () => clearTimeout(timer);
  }, [key, pending]);

  return {
    view,
    active: Boolean(incoming),
    pending,
    failed,
    objectFailed,
    showPending: pending && noticeKey === key,
    retry: () => setAttempt((value) => value + 1),
  };
}
