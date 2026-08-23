import { useCallback, useEffect, useRef, useState } from 'react';

/**
 * P025：复制到剪贴板 + 「已复制」反馈的共享 hook。
 * 收敛此前散落 7 处的 clipboard.writeText + copied 状态 + 定时复位样板。
 *
 * - `copy(text, key?)`：写入剪贴板（含旧浏览器 execCommand fallback），
 *   成功后标记 key 为已复制并在 resetMs 后自动复位；返回是否成功。
 * - `isCopied(key?)`：查询当前是否处于已复制态。
 * key 机制支持单组件内多处独立复制目标（字段名 / 数组下标 / addr+pin 等）。
 */
export function useCopyToClipboard(resetMs = 1500) {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const timersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  useEffect(() => {
    const timers = timersRef.current;
    return () => {
      Object.values(timers).forEach(clearTimeout);
    };
  }, []);

  const markCopied = useCallback(
    (key: string) => {
      setCopiedKey(key);
      if (timersRef.current[key]) clearTimeout(timersRef.current[key]);
      timersRef.current[key] = setTimeout(() => {
        setCopiedKey((prev) => (prev === key ? null : prev));
        delete timersRef.current[key];
      }, resetMs);
    },
    [resetMs],
  );

  const copy = useCallback(
    async (text: string, key = '__default__'): Promise<boolean> => {
      try {
        await navigator.clipboard.writeText(text);
        markCopied(key);
        return true;
      } catch {
        // 旧浏览器/非安全上下文 fallback
        try {
          const textarea = document.createElement('textarea');
          textarea.value = text;
          textarea.style.position = 'fixed';
          textarea.style.opacity = '0';
          document.body.appendChild(textarea);
          textarea.select();
          document.execCommand('copy');
          document.body.removeChild(textarea);
          markCopied(key);
          return true;
        } catch {
          return false;
        }
      }
    },
    [markCopied],
  );

  const isCopied = useCallback(
    (key = '__default__') => copiedKey === key,
    [copiedKey],
  );

  return { copy, isCopied, copiedKey };
}
