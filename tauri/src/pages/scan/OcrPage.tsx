import { useEffect, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useToastError } from '@/hooks/useToastError';
import { commands, type OcrResult, type OcrTierInfo, type OcrModelStatus, type MrzResult } from '@/lib/ipc';
import { OCR_MODEL_SERIES, OCR_MODEL_NOT_INSTALLED_PREFIX } from '@/lib/constants';
import { getTierLabel } from '@/lib/ocr';
import { MrzResultCard } from '@/components/ocr/MrzResultCard';
import { Scan, FileText, Upload, Download, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';


type ScanMode = 'general' | 'mrz';

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
  const [mrzResult, setMrzResult] = useState<MrzResult | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [scanMode, setScanMode] = useState<ScanMode>('general');

  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  const [activeTier, setActiveTier] = useState('small');
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [installingTier, setInstallingTier] = useState<string | null>(null);
  const [downloadingTier, setDownloadingTier] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState('');

  /** 处理扫描错误：将后端返回的「模型未安装」前缀解析为国际化提示。 */
  const handleScanError = (err: unknown) => {
    const message = err instanceof Error ? err.message : String(err);
    if (message.startsWith(`${OCR_MODEL_NOT_INSTALLED_PREFIX}:`)) {
      const tier = message.slice(OCR_MODEL_NOT_INSTALLED_PREFIX.length + 1) || activeTier;
      onError(new Error(t('ocr:scan_model_not_installed', { tier })), t('ocr:scan_failed'));
      return;
    }
    onError(err, t('ocr:scan_failed'));
  };

  /** 如果已知当前档位模型未安装，直接显示国际化提示。 */
  const guardActiveModelInstalled = (): boolean => {
    const status = statusMap[activeTier];
    if (status && !status.installed) {
      onError(new Error(t('ocr:scan_model_not_installed', { tier: activeTier })), t('ocr:scan_failed'));
      return false;
    }
    return true;
  };

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

  const getFileFilters = () => {
    if (scanMode === 'mrz') {
      return [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }];
    }
    return [{ name: 'Images & PDFs', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff', 'pdf'] }];
  };

  const performScan = async (path: string) => {
    if (!guardActiveModelInstalled()) return;
    setIsScanning(true);
    setResult(null);
    setMrzResult(null);
    try {
      if (scanMode === 'mrz') {
        const res = await commands.ocrScanMrz(path);
        if (res) {
          setMrzResult(res);
        } else {
          // MRZ not detected: fall back to general OCR
          const ocrRes = await commands.ocrScanImage(path);
          setResult(ocrRes);
          onSuccess(t('ocr:mrz_no_detected'));
        }
      } else {
        const res = await commands.ocrScanImage(path);
        setResult(res);
      }
    } catch (e) {
      handleScanError(e);
    } finally {
      setIsScanning(false);
    }
  };

  // 如果通过附件菜单传入文件路径，自动开始扫描。
  useEffect(() => {
    if (!initialFilePath) return;
    let cancelled = false;
    async function scan() {
      if (!guardActiveModelInstalled()) return;
      setIsScanning(true);
      setResult(null);
      setMrzResult(null);
      try {
        if (scanMode === 'mrz') {
          const res = await commands.ocrScanMrz(initialFilePath);
          if (cancelled) return;
          if (res) {
            setMrzResult(res);
          } else {
            const ocrRes = await commands.ocrScanImage(initialFilePath);
            if (!cancelled) setResult(ocrRes);
          }
        } else {
          const res = await commands.ocrScanImage(initialFilePath);
          if (!cancelled) setResult(res);
        }
      } catch (e) {
        if (!cancelled) handleScanError(e);
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

  const handleSelectFile = async () => {
    try {
      const path = await open({
        filters: getFileFilters(),
        multiple: false,
        title: scanMode === 'mrz' ? t('ocr:select_image_title') : t('ocr:select_file_title'),
      });
      if (path && typeof path === 'string') {
        setFilePath(path);
        await performScan(path);
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
    <AppShell title={t('ocr:title')} onBack={() => {
            const state = location.state as { fromHome?: boolean } | undefined;
            if (state?.fromHome) {
              navigate('/home');
            } else {
              navigate(-1);
            }
          }}>
      <PageContainer variant="wide" gap="default">
        {/* Model management */}
        <Card>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>{t('ocr:model_title')}</h3>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <label
                style={{
                  display: 'block',
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-secondary)',
                  marginBottom: 6,
                }}
              >
                {t('ocr:active_model_series', { model: OCR_MODEL_SERIES })}
              </label>
              <select
                value={activeTier}
                onChange={(e) => handleTierChange(e.target.value)}
                disabled={loadingStatus}
                style={{
                  width: '100%',
                  padding: '8px 10px',
                  fontSize: 'var(--text-body-sm)',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                }}
              >
                {tiers.map((tier) => {
                  const label = getTierLabel(t, tier);
                  return (
                    <option key={tier.tier} value={tier.tier}>
                      {label.name} — {label.description}
                    </option>
                  );
                })}
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
                        <CheckCircle size={ICON_SIZE.md} color="var(--accent-primary)" />
                      ) : status?.bundled ? (
                        <AlertCircle size={ICON_SIZE.md} color="var(--text-tertiary)" />
                      ) : (
                        <AlertCircle size={ICON_SIZE.md} color="var(--error)" />
                      )}
                      <div>
                        <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                          {getTierLabel(t, tier).name}
                        </div>
                        <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
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
                          <Download size={ICON_SIZE.sm} style={{ marginRight: 4 }} />
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
                    fontSize: 'var(--text-caption)',
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
                    fontSize: 'var(--text-body-sm)',
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

        {/* Scan mode + action */}
        <Card>
          <div style={{ textAlign: 'center', padding: 24 }}>
            <Scan
              size={ICON_SIZE['5xl']}
              style={{ marginBottom: 12, opacity: 0.3, color: 'var(--text-tertiary)' }}
            />
            <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 600, marginBottom: 4 }}>{t('ocr:title')}</h2>
            <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', marginBottom: 16 }}>
              {t('ocr:description')}
            </p>

            {/* Mode toggle */}
            <div
              style={{
                display: 'inline-flex',
                gap: 4,
                padding: 4,
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                marginBottom: 16,
              }}
            >
              <button
                onClick={() => {
                  setScanMode('general');
                  setResult(null);
                  setMrzResult(null);
                }}
                style={{
                  padding: '6px 14px',
                  borderRadius: 6,
                  border: 'none',
                  fontSize: 'var(--text-body-sm)',
                  cursor: 'pointer',
                  background: scanMode === 'general' ? 'var(--bg-elevated)' : 'transparent',
                  color: 'var(--text-primary)',
                  fontWeight: scanMode === 'general' ? 600 : 400,
                }}
              >
                {t('ocr:scan_mode_general')}
              </button>
              <button
                onClick={() => {
                  setScanMode('mrz');
                  setResult(null);
                  setMrzResult(null);
                }}
                style={{
                  padding: '6px 14px',
                  borderRadius: 6,
                  border: 'none',
                  fontSize: 'var(--text-body-sm)',
                  cursor: 'pointer',
                  background: scanMode === 'mrz' ? 'var(--bg-elevated)' : 'transparent',
                  color: 'var(--text-primary)',
                  fontWeight: scanMode === 'mrz' ? 600 : 400,
                }}
              >
                {t('ocr:scan_mode_mrz')}
              </button>
            </div>

            <br />

            <Button onClick={handleSelectFile} loading={isScanning}>
              <FileText size={ICON_SIZE.sm} style={{ marginRight: 6 }} />{' '}
              {scanMode === 'mrz' ? t('ocr:select_image') : t('ocr:select_image_or_pdf')}
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
              <Loader2 size={ICON_SIZE.lg} className="spin" />
              <span>{t('ocr:scanning')}</span>
            </div>
          </Card>
        )}

        {/* General OCR result */}
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
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>{t('ocr:result_title')}</h3>
              <Button size="sm" onClick={handleImportAsObject} loading={isImporting}>
                <Upload size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> {t('ocr:import_as_object')}
              </Button>
            </div>

            {result.boxes.length > 1 && (
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
                      fontSize: 'var(--text-body-sm)',
                    }}
                  >
                    <span style={{ flex: 1, wordBreak: 'break-word' }}>{box.text}</span>
                    <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', flexShrink: 0 }}>
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
                  fontSize: 'var(--text-body-sm)',
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
                  fontSize: 'var(--text-body-sm)',
                }}
              >
                {t('ocr:no_text')}
              </p>
            )}
          </Card>
        )}

        {/* MRZ result */}
        {mrzResult && (
          <Card>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: 12,
              }}
            >
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>{t('ocr:mrz_result_title')}</h3>
              <Button size="sm" onClick={handleImportAsObject} loading={isImporting}>
                <Upload size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> {t('ocr:import_as_object')}
              </Button>
            </div>
            <MrzResultCard result={mrzResult} />
          </Card>
        )}
      </PageContainer>
    </AppShell>
  );
}
