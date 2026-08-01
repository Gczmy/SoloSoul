import { useTranslation } from 'react-i18next';
import { Loader2, Wifi } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import type { RecoveryDiscoveredHost } from '@/components/recovery/recoveryReceiveTypes';

interface RecoveryManualViewProps {
  loading: boolean;
  scanning: boolean;
  discoveredHosts: RecoveryDiscoveredHost[];
  scanError: string | null;
  scanDone: boolean;
  hostAddr: string;
  pin: string;
  fingerprint: string;
  showAdvanced: boolean;
  error: string | null;
  onHostAddrChange: (v: string) => void;
  onPinChange: (v: string) => void;
  onFingerprintChange: (v: string) => void;
  onToggleAdvanced: () => void;
  onScanLan: () => void;
  onSelectHost: (host: RecoveryDiscoveredHost) => void;
  onNext: () => void;
}

/** 手动输入 tab：无摄像头设备兜底。 */
export function RecoveryManualView({
  loading,
  scanning,
  discoveredHosts,
  scanError,
  scanDone,
  hostAddr,
  pin,
  fingerprint,
  showAdvanced,
  error,
  onHostAddrChange,
  onPinChange,
  onFingerprintChange,
  onToggleAdvanced,
  onScanLan,
  onSelectHost,
  onNext,
}: RecoveryManualViewProps) {
  const { t } = useTranslation(['common']);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          margin: '0 0 4px',
          lineHeight: 1.5,
        }}
      >
        {t('common:recovery_receive_desc')}
      </p>

      {/* ── 局域网扫描 ── */}
      <div
        style={{
          padding: '10px 12px',
          borderRadius: 8,
          border: '1px dashed var(--border-subtle)',
          marginBottom: 8,
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: discoveredHosts.length > 0 || scanError ? 8 : 0,
          }}
        >
          <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
            {t('common:recovery_scan_lan_label', { defaultValue: 'LAN Discovery' })}
          </span>
          <button
            type="button"
            onClick={onScanLan}
            disabled={scanning || loading}
            className="interactive-accent-soft"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '5px 10px',
              borderRadius: 6,
              borderWidth: 1,
              borderStyle: 'solid',
              borderColor: scanning ? 'var(--border-subtle)' : 'transparent',
              background: scanning ? 'var(--bg-toolbar)' : undefined,
              color: scanning ? 'var(--text-tertiary)' : undefined,
              cursor: scanning || loading ? 'not-allowed' : 'pointer',
              fontFamily: 'inherit',
              fontSize: 'var(--text-caption)',
              fontWeight: 500,
              opacity: scanning || loading ? 0.6 : 1,
            }}
          >
            {scanning ? (
              <Loader2 size={14} style={{ animation: 'spin 1s linear infinite' }} />
            ) : (
              <Wifi size={14} />
            )}
            {scanning
              ? t('common:recovery_scan_scanning', { defaultValue: 'Scanning…' })
              : t('common:recovery_scan_button', { defaultValue: 'Scan LAN' })}
          </button>
        </div>

        {/* 发现的设备列表 */}
        {discoveredHosts.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {discoveredHosts.map((host, i) => (
              <button
                key={`${host.addr}-${i}`}
                type="button"
                onClick={() => onSelectHost(host)}
                disabled={loading}
                className="interactive-outline"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: '8px 10px',
                  borderRadius: 6,
                  borderWidth: 1,
                  borderStyle: 'solid',
                  background: 'var(--bg-elevated)',
                  cursor: loading ? 'not-allowed' : 'pointer',
                  fontFamily: 'inherit',
                  textAlign: 'left',
                  opacity: loading ? 0.6 : 1,
                }}
              >
                <div style={{ minWidth: 0 }}>
                  <div
                    style={{
                      fontSize: 'var(--text-body-sm)',
                      fontWeight: 500,
                      color: 'var(--text-primary)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {host.name}
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-tertiary)',
                      fontFamily: 'monospace',
                    }}
                  >
                    {host.addr}
                  </div>
                </div>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                    padding: '2px 6px',
                    borderRadius: 4,
                    background: 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                    color: 'var(--accent-primary)',
                    fontSize: 'var(--text-caption)',
                    fontWeight: 500,
                  }}
                >
                  {t('common:recovery_need_pin', {
                    defaultValue: '需输入 PIN',
                  })}
                </div>
              </button>
            ))}
          </div>
        )}

        {scanError && !scanning && (
          <div
            style={{
              fontSize: 'var(--text-caption)',
              color: scanDone && discoveredHosts.length === 0
                ? 'var(--text-tertiary)'
                : '#e74c3c',
              padding: '2px 0',
            }}
          >
            {scanError}
          </div>
        )}
      </div>

      <Input
        label={t('common:recovery_receive_addr_label')}
        type="text"
        value={hostAddr}
        onChange={(e) => onHostAddrChange(e.target.value)}
        placeholder={t('common:recovery_receive_addr_placeholder')}
        disabled={loading}
      />

      <Input
        label={t('common:recovery_receive_pin_label')}
        type="text"
        value={pin}
        onChange={(e) => onPinChange(e.target.value.replace(/\D/g, '').slice(0, 6))}
        placeholder="123456"
        maxLength={6}
        disabled={loading}
        style={{ fontFamily: 'monospace', letterSpacing: 4, fontSize: 'var(--text-body)' }}
      />

      {/* 展开/收起高级选项（指纹） */}
      <button
        type="button"
        onClick={onToggleAdvanced}
        disabled={loading}
        className="interactive-accent-link"
        style={{
          background: 'none',
          border: 'none',
          fontSize: 'var(--text-caption)',
          cursor: loading ? 'not-allowed' : 'pointer',
          fontFamily: 'inherit',
          padding: '2px 0',
          textAlign: 'left',
        }}
      >
        {showAdvanced
          ? t('common:recovery_advanced_hide', { defaultValue: 'Hide optional fingerprint' })
          : t('common:recovery_advanced_show', { defaultValue: 'Show optional fingerprint' })}
      </button>

      {showAdvanced && (
        <Input
          label={t('common:recovery_receive_fingerprint_label')}
          type="text"
          value={fingerprint}
          onChange={(e) => onFingerprintChange(e.target.value)}
          placeholder={t('common:recovery_fingerprint_placeholder', { defaultValue: 'e.g. abc123…' })}
          disabled={loading}
        />
      )}

      <Button
        onClick={onNext}
        disabled={loading}
        style={{ width: '100%', marginTop: 4 }}
      >
        {t('common:next')}
      </Button>

      {error && (
        <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
      )}
    </div>
  );
}
