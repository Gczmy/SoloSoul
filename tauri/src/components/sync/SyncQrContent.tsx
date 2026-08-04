import { QRCodeSVG } from 'qrcode.react';
import { motion } from 'framer-motion';
import type { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { QrStatusBlock } from './QrStatusBlock';

/** 同步配对二维码数据（sync_generate_qr_payload 解析结果）。 */
export interface SyncQrInfo {
  payload: string;
  addr: string;
  fingerprint: string;
  deviceName: string;
}

type T = ReturnType<typeof useTranslation>['t'];

/** 同步配对二维码内容区：标题 + 加载/错误占位 + 二维码与设备信息。 */
export function SyncQrContent({
  t,
  loading,
  error,
  info,
  onClose,
}: {
  t: T;
  loading: boolean;
  error: string | null;
  info: SyncQrInfo | null;
  onClose: () => void;
}) {
  return (
    <>
      <h2
        style={{
          fontSize: 'var(--text-card-title)',
          fontWeight: 700,
          margin: '0 0 8px',
          color: 'var(--text-primary)',
        }}
      >
        {t('common:sync_qr_show_title')}
      </h2>
      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          margin: '0 0 20px',
          lineHeight: 1.5,
        }}
      >
        {t('common:sync_qr_show_desc')}
      </p>

      <QrStatusBlock loading={loading} error={error} t={t} />

      {!loading && !error && info && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 16,
            alignItems: 'center',
            minHeight: 360,
          }}
        >
          <div
            style={{
              padding: 12,
              background: '#fff',
              borderRadius: 12,
              border: '1px solid var(--border-subtle)',
            }}
          >
            <QRCodeSVG value={info.payload} size={200} level="M" includeMargin />
          </div>

          <div style={{ width: '100%' }}>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '10px 12px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                marginBottom: 8,
              }}
            >
              <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                {t('common:sync_qr_device_name')}
              </span>
              <span
                style={{
                  fontSize: 'var(--text-body-sm)',
                  fontWeight: 500,
                  color: 'var(--text-primary)',
                }}
              >
                {info.deviceName}
              </span>
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '10px 12px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                marginBottom: 8,
              }}
            >
              <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                {t('common:sync_qr_addr')}
              </span>
              <span
                style={{
                  fontFamily: 'monospace',
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-primary)',
                }}
              >
                {info.addr}
              </span>
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '10px 12px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
              }}
            >
              <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                {t('common:sync_qr_fingerprint')}
              </span>
              <span
                style={{
                  fontFamily: 'monospace',
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-primary)',
                  wordBreak: 'break-all',
                  maxWidth: '60%',
                  textAlign: 'right',
                }}
              >
                {info.fingerprint}
              </span>
            </div>
          </div>

          {info.addr.startsWith('127.') && (
            <p
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--warning)',
                textAlign: 'center',
                margin: 0,
              }}
            >
              {t('common:sync_qr_localhost_warning')}
            </p>
          )}

          <Button variant="secondary" onClick={onClose} style={{ width: '100%' }}>
            {t('common:close')}
          </Button>
        </motion.div>
      )}
    </>
  );
}
