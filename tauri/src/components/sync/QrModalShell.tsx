import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { X } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import type { ReactNode } from 'react';

interface QrModalShellProps {
  onClose: () => void;
  /** 可选标题（渲染为 h2）。 */
  title?: string;
  /** 内容超出视口时卡片内滚动（ShowQrDialog 需要）。 */
  scrollable?: boolean;
  children: ReactNode;
}

/**
 * QR 对话框共享模态外壳（P042：SyncScanQrDialog 与 SyncShowQrDialog 的手写模态外壳
 * 一致——overlay + 淡入卡片 + 右上关闭按钮）。
 */
export function QrModalShell({ onClose, title, scrollable = false, children }: QrModalShellProps) {
  const { t } = useTranslation(['common']);

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
        padding: 16,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {/* 卡片进场淡入：消除手写模态的硬弹出闪烁（与共享 Dialog 的 dialogIn 动画对齐） */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.2 }}
        style={{ width: '100%', maxWidth: 420 }}
      >
        <Card
          style={{
            // 宽度由外层 motion.div（width:100% + maxWidth:420）约束，这里只填满，避免双重 maxWidth
            width: '100%',
            padding: 24,
            position: 'relative',
            // 与 Dialog 组件一致：展开「手动模式」后内容较高，超出视口时允许卡片内滚动，
            // 避免 flex 居中溢出导致上下内容（tab 切换/关闭/取消按钮）不可达。
            // 注意：必须用视口单位 100vh 而非百分比 100% —— 父级 motion.div 高度为 auto，
            // 百分比无法解析会使整个 min() 失效，导致 max-height 不生效、内容全高溢出。
            ...(scrollable
              ? { maxHeight: 'min(85vh, calc(100vh - 32px))', overflowY: 'auto' as const }
              : {}),
          }}
        >
          <button
            type="button"
            onClick={onClose}
            style={{
              position: 'absolute',
              top: 12,
              right: 12,
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
            }}
            aria-label={t('common:close')}
          >
            <X size={20} />
          </button>

          {title && (
            <h2
              style={{
                fontSize: 'var(--text-card-title)',
                fontWeight: 700,
                margin: '0 0 8px',
                color: 'var(--text-primary)',
              }}
            >
              {title}
            </h2>
          )}

          {children}
        </Card>
      </motion.div>
    </div>
  );
}
