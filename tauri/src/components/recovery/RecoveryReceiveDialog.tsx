import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { useRecoveryReceive } from '@/hooks/useRecoveryReceive';
import { RecoveryReceiveTabs } from '@/components/recovery/RecoveryReceiveTabs';
import { RecoverySuccessView } from '@/components/recovery/RecoverySuccessView';
import { RecoveryAccountView } from '@/components/recovery/RecoveryAccountView';
import { RecoveryScanView } from '@/components/recovery/RecoveryScanView';
import { RecoveryManualView } from '@/components/recovery/RecoveryManualView';

interface RecoveryReceiveDialogProps {
  isOpen: boolean;
  onClose: () => void;
  /** 恢复成功后调用；若提供则替代默认的 /home 导航 */
  onSuccess?: () => void;
}

/**
 * 从其他设备接收恢复包（新设备）：
 * - collect 阶段：扫码 / 手动输入获取连接信息（含局域网发现）
 * - account 阶段：确认账户信息 + 设置本机主密码
 * - success 阶段：显示导入统计
 * 状态机与业务逻辑全部收敛在 useRecoveryReceive hook，本组件仅编排视图。
 */
export function RecoveryReceiveDialog({ isOpen, onClose, onSuccess }: RecoveryReceiveDialogProps) {
  const { t } = useTranslation(['common']);
  const rcv = useRecoveryReceive({ isOpen, onClose, onSuccess });

  if (!isOpen) return null;

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
        if (e.target === e.currentTarget) rcv.handleClose();
      }}
    >
      <Card
        style={{
          maxWidth: 420,
          width: '100%',
          padding: 24,
          position: 'relative',
        }}
      >
        <button
          type="button"
          onClick={rcv.handleClose}
          style={{
            position: 'absolute',
            top: 12,
            right: 12,
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--text-tertiary)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
          aria-label={t('common:close')}
        >
          <X size={20} />
        </button>

        <h2
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 700,
            margin: '0 0 8px',
            color: 'var(--text-primary)',
            paddingRight: 24,
          }}
        >
          {t('common:recovery_receive_title')}
        </h2>

        {/* Tab 切换（仅收集连接信息阶段显示） */}
        {rcv.step === 'collect' && (
          <RecoveryReceiveTabs tab={rcv.tab} loading={rcv.loading} onSwitch={rcv.switchTab} />
        )}

        {rcv.step === 'success' && rcv.success ? (
          <>
            <RecoverySuccessView
              success={rcv.success}
              onComplete={() => rcv.setSuccessConfirmOpen(true)}
            />
            {/* 恢复完成确认框：用户确认后返回登录页（登录页展示刚恢复的账户） */}
            <ConfirmDialog
              isOpen={rcv.successConfirmOpen}
              title={t('common:recovery_complete_title', { defaultValue: 'Recovery complete' })}
              message={t('common:recovery_complete_desc', {
                objects: rcv.success.objectCount,
                attachments: rcv.success.attachmentCount,
                defaultValue:
                  'Recovery completed. Return to the login page and unlock with your new password.',
              })}
              confirmLabel={t('common:confirm')}
              cancelLabel={t('common:cancel')}
              confirmVariant="primary"
              onConfirm={rcv.handleClose}
              onCancel={() => rcv.setSuccessConfirmOpen(false)}
            />
          </>
        ) : rcv.step === 'account' && rcv.pending ? (
          <RecoveryAccountView
            pending={rcv.pending}
            loading={rcv.loading}
            statusText={rcv.statusText}
            progress={rcv.progress}
            masterPassword={rcv.masterPassword}
            confirmPassword={rcv.confirmPassword}
            passwordHint={rcv.passwordHint}
            masterPasswordError={rcv.masterPasswordError}
            confirmPasswordError={rcv.confirmPasswordError}
            error={rcv.error}
            onMasterPasswordChange={rcv.handleMasterPasswordChange}
            onConfirmPasswordChange={rcv.handleConfirmPasswordChange}
            onPasswordHintChange={rcv.setPasswordHint}
            onStartRecovery={rcv.handleStartRecovery}
            onBackToCollect={rcv.handleBackToCollect}
            idConflict={rcv.idConflict}
            overwriteApproved={rcv.overwriteApproved}
            confirmingOverwrite={rcv.confirmingOverwrite}
            onRequestOverwrite={rcv.handleRequestOverwrite}
            onCancelConflict={rcv.handleCancelConflict}
            onCancelOverwriteConfirm={rcv.handleCancelOverwriteConfirm}
            onConfirmOverwrite={rcv.handleOverwriteRecovery}
          />
        ) : rcv.tab === 'scan' ? (
          <RecoveryScanView
            cameraCapability={rcv.cameraCapability}
            scannerError={rcv.scannerError}
            error={rcv.error}
            onScan={rcv.handleScan}
            onScannerError={rcv.setScannerError}
            onSwitchManual={() => rcv.switchTab('manual')}
          />
        ) : (
          <RecoveryManualView
            loading={rcv.loading}
            scanning={rcv.scanning}
            discoveredHosts={rcv.discoveredHosts}
            scanError={rcv.scanError}
            scanDone={rcv.scanDone}
            hostAddr={rcv.hostAddr}
            pin={rcv.pin}
            fingerprint={rcv.fingerprint}
            showAdvanced={rcv.showAdvanced}
            error={rcv.error}
            onHostAddrChange={rcv.setHostAddr}
            onPinChange={rcv.setPin}
            onFingerprintChange={rcv.setFingerprint}
            onToggleAdvanced={() => rcv.setShowAdvanced(!rcv.showAdvanced)}
            onScanLan={rcv.handleScanLan}
            onSelectHost={rcv.handleSelectHost}
            onNext={rcv.handleManualNext}
          />
        )}
      </Card>
    </div>
  );
}
