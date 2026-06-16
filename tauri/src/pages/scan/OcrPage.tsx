import { useEffect, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useToastError } from '@/hooks/useToastError';
import { commands, type OcrResult, type OcrTierInfo, type OcrModelStatus } from '@/lib/ipc';
import { Scan, FileText, Upload, Download, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';

export function OcrPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['ocr', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { createObject } = useObjectStore();
  const { onError, onSuccess } = useToastError();

  const initialFilePath = (location.state as { filePath?: string } | null)?.filePath || '';

  const [filePath, setFilePath] = useState(initialFilePath);
  const [result, setResult] = useState<OcrResult | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  const [activeTier, setActiveTier] = useState('small');
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [installingTier, setInstallingTier] = useState<string | null>(null);
  const [downloadingTier, setDownloadingTier] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState('');

  const loadTiersAndStatus = async () => {
    try {
      setLoadingStatus(true);
      const [tierList, currentTier] = await Promise.all([
        commands.ocrListAvailableTiers(),
        commands.ocrGetActiveTier(),
      ]);
      setTiers(tierList);
      setActiveTier(currentTier);

      const statuses: Record<string, OcrModelStatus> = {};
      await Promise.all(
        tierList.map(async (tier) => {
          const status = await commands.ocrGetModelStatus(tier.tier);
          statuses[tier.tier] = status;
        }),
      );
      setStatusMap(statuses);
    } catch (e) {
      onError(e, t('ocr:load_status_failed'));
    } finally {
      setLoadingStatus(false);
    }
  };

  useEffect(() => {
    loadTiersAndStatus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 如果通过附件菜单传入文件路径，自动开始扫描。
  useEffect(() => {
    if (!initialFilePath) return;
    let cancelled = false;
    async function scan() {
      setIsScanning(true);
      setResult(null);
      try {
        const res = await commands.ocrScanImage(initialFilePath);
        if (cancelled) return;
        setResult(res);
      } catch (e) {
        if (!cancelled) onError(e, t('ocr:scan_failed'));
      } finally {
        if (!cancelled) setIsScanning(false);
      }
    }
    scan();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialFilePath]);

  const handleSelectImage = async () => {
    try {
      const path = await open({
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }],
        multiple: false,
        title: t('ocr:select_image_title'),
      });
      if (path && typeof path === 'string') {
        setFilePath(path);
        setIsScanning(true);
        setResult(null);
        try {
          const res = await commands.ocrScanImage(path);
          setResult(res);
        } catch (e) {
          onError(e, t('ocr:scan_failed'));
        } finally {
          setIsScanning(false);
        }
      }
    } catch (e) {
      onError(e, t('ocr:select_image_failed'));
    }
  };

  const handleImportAsObject = async () => {
    if (!accountId || !result) return;
    setIsImporting(true);
    try {
      await createObject({
        accountId,
        name: filePath.split('/').pop() || t('ocr:scanned_document'),
        collectionType: 'document',
        properties: { ocrText: result.text },
      });
      onSuccess(t('ocr:import_success'));
    } catch (e) {
      onError(e, t('ocr:import_failed'));
    } finally {
      setIsImporting(false);
    }
  };

  const handleTierChange = async (tier: string) => {
    try {
      await commands.ocrSetActiveTier(tier);
      setActiveTier(tier);
    } catch (e) {
      onError(e, t('ocr:set_tier_failed'));
    }
  };

  const handleInstallBundled = async (tier: string) => {
    setInstallingTier(tier);
    try {
      await commands.ocrInstallBundledModel(tier);
      await loadTiersAndStatus();
      onSuccess(t('ocr:install_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:install_failed', { tier }));
    } finally {
      setInstallingTier(null);
    }
  };

  const handleDownload = async (tier: string) => {
    if (!downloadUrl.trim()) {
      onError(new Error(t('ocr:download_url_required')), t('ocr:download_url_required'));
      return;
    }
    setDownloadingTier(tier);
    try {
      await commands.ocrDownloadModel(tier, downloadUrl.trim());
      await loadTiersAndStatus();
      onSuccess(t('ocr:download_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:download_failed', { tier }));
    } finally {
      setDownloadingTier(null);
    }
  };

  return (
    <AppShell title={t('ocr:title')} onBack={() => navigate(-1)}>
      <div
        style={{
          maxWidth: 720,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        {/* Model management */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{t('ocr:model_title')}</h3>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <label
                style={{
                  display: 'block',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                  marginBottom: 6,
                }}
              >
                {t('ocr:active_model')}
              </label>
              <select
                value={activeTier}
                onChange={(e) => handleTierChange(e.target.value)}
                disabled={loadingStatus}
                style={{
                  width: '100%',
                  padding: '8px 10px',
                  fontSize: 13,
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                }}
              >
                {tiers.map((tier) => (
                  <option key={tier.tier} value={tier.tier}>
                    {tier.name} — {tier.description}
                  </option>
                ))}
              </select>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {tiers.map((tier) => {
                const status = statusMap[tier.tier];
                const isInstalling = installingTier === tier.tier;
                const isDownloading = downloadingTier === tier.tier;
                return (
                  <div
                    key={tier.tier}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      padding: '10px 12px',
                      borderRadius: 8,
                      background: 'var(--bg-toolbar)',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
                      {status?.installed ? (
                        <CheckCircle size={16} color="var(--accent-primary)" />
                      ) : status?.bundled ? (
                        <AlertCircle size={16} color="var(--text-tertiary)" />
                      ) : (
                        <AlertCircle size={16} color="var(--error)" />
                      )}
                      <div>
                        <div style={{ fontSize: 13, fontWeight: 500 }}>{tier.name}</div>
                        <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                          {status?.installed
                            ? t('ocr:status_installed')
                            : status?.bundled
                              ? t('ocr:status_bundled')
                              : t('ocr:status_not_installed')}
                        </div>
                      </div>
                    </div>
                    <div style={{ display: 'flex', gap: 8 }}>
                      {status?.bundled && !status?.installed && (
                        <Button
                          size="sm"
                          onClick={() => handleInstallBundled(tier.tier)}
                          loading={isInstalling}
                        >
                          {t('ocr:install')}
                        </Button>
                      )}
                      {!status?.bundled && !status?.installed && (
                        <Button
                          size="sm"
                          onClick={() => handleDownload(tier.tier)}
                          loading={isDownloading}
                        >
                          <Download size={14} style={{ marginRight: 4 }} />
                          {t('ocr:download')}
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            {!statusMap['small']?.installed && !statusMap['small']?.bundled && (
              <div>
                <label
                  style={{
                    display: 'block',
                    fontSize: 12,
                    color: 'var(--text-secondary)',
                    marginBottom: 6,
                  }}
                >
                  {t('ocr:download_url_label')}
                </label>
                <input
                  type="text"
                  value={downloadUrl}
                  onChange={(e) => setDownloadUrl(e.target.value)}
                  placeholder={t('ocr:download_url_placeholder')}
                  style={{
                    width: '100%',
                    padding: '8px 10px',
                    fontSize: 13,
                    borderRadius: 8,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-elevated)',
                    color: 'var(--text-primary)',
                  }}
                />
              </div>
            )}
          </div>
        </Card>

        {/* Scan area */}
        <Card>
          <div style={{ textAlign: 'center', padding: 24 }}>
            <Scan
              size={48}
              style={{ marginBottom: 12, opacity: 0.3, color: 'var(--text-tertiary)' }}
            />
            <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 4 }}>{t('ocr:title')}</h2>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 16 }}>
              {t('ocr:description')}
            </p>
            <Button onClick={handleSelectImage} loading={isScanning}>
              <FileText size={14} style={{ marginRight: 6 }} /> {t('ocr:select_image')}
            </Button>
          </div>
        </Card>

        {isScanning && (
          <Card>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 10,
                padding: 24,
                color: 'var(--text-secondary)',
              }}
            >
              <Loader2 size={18} className="spin" />
              <span>{t('ocr:scanning')}</span>
            </div>
          </Card>
        )}

        {result && (
          <Card>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: 12,
              }}
            >
              <h3 style={{ fontSize: 14, fontWeight: 600 }}>{t('ocr:result_title')}</h3>
              <Button size="sm" onClick={handleImportAsObject} loading={isImporting}>
                <Upload size={14} style={{ marginRight: 4 }} /> {t('ocr:import_as_object')}
              </Button>
            </div>

            {result.boxes.length > 0 && (
              <div style={{ marginBottom: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
                {result.boxes.map((box, i) => (
                  <div
                    key={i}
                    style={{
                      display: 'flex',
                      gap: 12,
                      padding: '8px 10px',
                      borderRadius: 6,
                      background: 'var(--bg-toolbar)',
                      fontSize: 13,
                    }}
                  >
                    <span style={{ flex: 1, wordBreak: 'break-word' }}>{box.text}</span>
                    <span style={{ fontSize: 11, color: 'var(--text-tertiary)', flexShrink: 0 }}>
                      {(box.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                ))}
              </div>
            )}

            {result.text && (
              <div
                style={{
                  padding: 12,
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  fontSize: 13,
                  lineHeight: 1.6,
                  whiteSpace: 'pre-wrap',
                  maxHeight: 300,
                  overflowY: 'auto',
                }}
              >
                {result.text}
              </div>
            )}

            {result.boxes.length === 0 && !result.text && (
              <p
                style={{
                  textAlign: 'center',
                  color: 'var(--text-tertiary)',
                  padding: 24,
                  fontSize: 13,
                }}
              >
                {t('ocr:no_text')}
              </p>
            )}
          </Card>
        )}
      </div>
    </AppShell>
  );
}
