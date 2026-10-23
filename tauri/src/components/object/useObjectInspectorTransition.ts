import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
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
  const [attempt, setAttempt] = useState(0);
  // 内容节点归属仅随对象/标签变化；保存刷新不重挂载、不清空正文和滚动位置。
  const key = JSON.stringify([context.objectId, tab]);
  const readKey = JSON.stringify([
    key,
    attempt,
    tab === 'history' ? (context.historyRevision ?? 0) : 0,
  ]);
  const [result, setResult] = useState<{
    key: string;
    data?: InspectorData;
    failed?: boolean;
  }>();
  const [displayed, setDisplayed] = useState<InspectorView>();
  const objectFailed = Boolean(context.failed);
  useLayoutEffect(() => {
    const request = requests.begin('metadata', accountId);
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
          if (request.isCurrent()) setResult({ key: readKey, data });
        })
        .catch(() => {
          if (request.isCurrent()) setResult({ key: readKey, failed: true });
        });
    }
    return () => requests.invalidate('metadata');
  }, [accountId, context.objectId, readKey, objectFailed, requests, tab]);
  useLayoutEffect(() => () => requests.invalidate(), [requests]);

  const currentResult = result?.key === readKey ? result : undefined;
  const retaining = Boolean(context.ready && !objectFailed && displayed?.key === key);
  const refreshFailed = retaining && Boolean(currentResult?.failed);
  const failed = objectFailed || Boolean(context.ready && currentResult?.failed && !retaining);
  const incoming = useMemo<InspectorView | undefined>(
    () =>
      context.ready && !objectFailed && currentResult?.data
        ? {
            key,
            context: {
              objectId: context.objectId,
              objectName: context.objectName,
              typeId: context.typeId,
              ready: true,
              historyRevision: context.historyRevision,
            },
            data: currentResult.data,
          }
        : undefined,
    [
      context.objectId,
      context.objectName,
      context.typeId,
      context.ready,
      context.historyRevision,
      objectFailed,
      currentResult,
      key,
    ],
  );
  useLayoutEffect(() => {
    if (failed) setDisplayed(undefined);
    else if (incoming) setDisplayed(incoming);
  }, [failed, incoming]);
  // 切换标签也交接完整画面，目标列表就绪前保留旧列表，避免短暂清空内容。
  const view = failed ? undefined : (incoming ?? displayed);
  const pending = !failed && !incoming && !currentResult?.failed;
  const [noticeKey, setNoticeKey] = useState<string>();
  useEffect(() => {
    if (!pending || retaining) {
      setNoticeKey(undefined);
      return;
    }
    const timer = setTimeout(() => setNoticeKey(readKey), 250);
    return () => clearTimeout(timer);
  }, [readKey, pending, retaining]);

  return {
    view,
    active: Boolean(incoming) || retaining,
    pending,
    failed,
    refreshFailed,
    objectFailed,
    showPending: pending && !retaining && noticeKey === readKey,
    retry: () => setAttempt((value) => value + 1),
  };
}
