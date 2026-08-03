import { useEffect, useState, useMemo } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useToastError } from '@/hooks/useToastError';
import { useOcrModelManager } from '@/hooks/useOcrModelManager';
import { isMobilePlatformSync } from '@/lib/platform';

import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { MrzResult, OcrResult } from '@/lib/ipc';
import { OCR_MODEL_NOT_INSTALLED_PREFIX } from '@/lib/constants';
import { Info, Import, Layers, Scan } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { OcrResultList } from './OcrResultList';
import { OcrScanSettingsPanel } from './OcrScanSettingsPanel';
import { ScanDropZone, type ScanMode } from './ScanDropZone';

export function OcrPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['ocr', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { createObject } = useObjectStore();
  const { onError, onSuccess } = useToastError();

  const initialFilePath = (location.state as { filePath?: string } | null)?.filePath || '';

  const [result, setResult] = useState<OcrResult | null>(null);
  const [mrzResult, setMrzResult] = useState<MrzResult | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isNameDialogOpen, setIsNameDialogOpen] = useState(false);
  const [importNameDefault, setImportNameDefault] = useState('');
  const [pendingImportSource, setPendingImportSource] = useState<'ocr' | 'mrz' | null>(null);
  const [scanMode, setScanMode] = useState<ScanMode>('general');

  const isMobilePlatform = isMobilePlatformSync();

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
      onError(
        new Error(t('ocr:scan_model_not_installed', { tier: activeTier })),
        t('ocr:scan_failed'),
      );
      return false;
    }
    return true;
  };

  const {
    tiers,
    activeTier,
    statusMap,
    loading: loadingStatus,
    installingTier,
    downloadingTier,
    downloadUrl,
    setDownloadUrl,
    handleTierChange,
    handleInstallBundled,
    handleDownload,
  } = useOcrModelManager({
    t,
    onError,
    onInstallSuccess: onSuccess,
    onDownloadSuccess: onSuccess,
  });

  const getFileFilters = () => {
    // MRZ、移动端与 macOS Vision 引擎均只支持图片格式
    // （移动端 ML Kit 无法处理 PDF；Vision 引擎无 PDF 渲染管线）
    if (scanMode === 'mrz' || isMobilePlatform || activeTier === 'vision') {
      return [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }];
    }
    return [
      { name: 'Images & PDFs', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff', 'pdf'] },
    ];
  };

  const performScan = async (path: string) => {
    if (!guardActiveModelInstalled()) return;
    setIsScanning(true);
    setResult(null);
    setMrzResult(null);
    try {
      if (scanMode === 'mrz') {
        const res = await invoke<MrzResult | null>('ocr_scan_mrz', { filePath: path });
        if (res) {
          setMrzResult(res);
        } else {
          // MRZ not detected: fall back to general OCR
          const ocrRes = await invoke<OcrResult>('ocr_scan_image', { filePath: path });
          setResult(ocrRes);
          onSuccess(t('ocr:mrz_no_detected'));
        }
      } else {
        const res = await invoke<OcrResult>('ocr_scan_image', { filePath: path });
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
          const res = await invoke<MrzResult | null>('ocr_scan_mrz', { filePath: initialFilePath });
          if (cancelled) return;
          if (res) {
            setMrzResult(res);
          } else {
            const ocrRes = await invoke<OcrResult>('ocr_scan_image', { filePath: initialFilePath });
            if (!cancelled) setResult(ocrRes);
          }
        } else {
          const res = await invoke<OcrResult>('ocr_scan_image', { filePath: initialFilePath });
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
    // P212: guardActiveModelInstalled/handleScanError/scanMode omitted intentionally —
    // they are stable functions; adding them would cause re-scan on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialFilePath]);

  const handleSelectFile = async () => {
    try {
      const { openWithPause } = await import('@/lib/dialog');
      const path = await openWithPause({
        filters: getFileFilters(),
        multiple: false,
        title:
          scanMode === 'mrz' || isMobilePlatform || activeTier === 'vision'
            ? t('ocr:select_image_title')
            : t('ocr:select_file_title'),
      });
      if (path && typeof path === 'string') {
        await performScan(path);
      }
    } catch (e) {
      onError(e, t('ocr:select_image_failed'));
    }
  };

  const handleTakePhoto = async () => {
    // 启动相机前立即清空上次结果并显示加载状态，避免从相机返回后闪烁旧结果
    setResult(null);
    setMrzResult(null);
    setIsScanning(true);
    try {
      const { useAutoLockPauseStore } = await import('@/stores/autoLockPauseStore');
      const { pause, resume } = useAutoLockPauseStore.getState();
      pause();
      try {
        const path = await invoke<string | null>('mobile_ocr_take_photo');
        if (path) {
          await performScan(path);
        } else {
          // 相机取消或未返回路径，恢复空闲状态
          setIsScanning(false);
          onError(t('ocr:take_photo_no_image'), t('ocr:take_photo_failed'));
        }
      } catch (e) {
        setIsScanning(false);
        onError(e, t('ocr:take_photo_failed'));
      } finally {
        resume();
      }
    } catch (e) {
      setIsScanning(false);
      onError(e, t('ocr:take_photo_failed'));
    }
  };

  /** 生成 OCR 导入对象的默认名称：前缀 + 当前日期时间（YYYYMMDDHHMMSS） */
  const generateDefaultImportName = () => {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    const hours = String(now.getHours()).padStart(2, '0');
    const minutes = String(now.getMinutes()).padStart(2, '0');
    const seconds = String(now.getSeconds()).padStart(2, '0');
    const datetime = `${year}${month}${day}${hours}${minutes}${seconds}`;
    return t('ocr:import_default_name', { datetime, defaultValue: `OCR扫描结果${datetime}` });
  };

  const handleImportAsObject = (source: 'ocr' | 'mrz') => {
    if (!accountId) return;
    if (source === 'ocr' && !result) return;
    if (source === 'mrz' && !mrzResult) return;
    setPendingImportSource(source);
    setImportNameDefault(generateDefaultImportName());
    setIsNameDialogOpen(true);
  };

  const buildImportProperties = () => {
    const ocrFieldName = t('ocr:field_ocr_text', { defaultValue: 'OCR 文本' });
    const __fields = {
      ocrText: {
        name: ocrFieldName,
        type: 'multiline',
        sensitivityLevel: 'internal',
      },
    };

    if (pendingImportSource === 'ocr' && result) {
      return { ocrText: result.text, __fields };
    }
    if (pendingImportSource === 'mrz' && mrzResult) {
      const summary = [
        `${t('ocr:mrz_field_type')}: ${mrzResult.documentType} (${mrzResult.documentTypeSub})`,
        `${t('ocr:mrz_field_country')}: ${mrzResult.issuingCountry}`,
        `${t('ocr:mrz_field_number')}: ${mrzResult.documentNumber}`,
        `${t('ocr:mrz_field_nationality')}: ${mrzResult.nationality}`,
        `${t('ocr:mrz_field_dob')}: ${mrzResult.dateOfBirth}`,
        `${t('ocr:mrz_field_sex')}: ${mrzResult.sex}`,
        `${t('ocr:mrz_field_expiry')}: ${mrzResult.expiryDate}`,
        `${t('ocr:mrz_raw_lines')}:\n${mrzResult.rawLines.join('\n')}`,
      ].join('\n');
      return { ocrText: summary, __fields };
    }
    return {};
  };

  const handleConfirmImport = async (name: string) => {
    if (!accountId || !pendingImportSource) return;
    setIsNameDialogOpen(false);
    setIsImporting(true);
    try {
      await createObject({
        accountId,
        name,
        collectionType: 'document',
        properties: buildImportProperties(),
      });
      onSuccess(t('ocr:import_success'));
    } catch (e) {
      onError(e, t('ocr:import_failed'));
    } finally {
      setIsImporting(false);
      setPendingImportSource(null);
    }
  };

  const ocrGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_ocr_title') ?? 'OCR Scan Guide',
        steps: [
          {
            icon: Scan,
            title: t('common:guide_ocr_step1_title') ?? 'Choose Input',
            description:
              t('common:guide_ocr_step1_desc') ??
              'Take a photo with the camera or select an image from your device. OCR extracts text from the image.',
          },
          {
            icon: Layers,
            title: t('common:guide_ocr_step2_title') ?? 'Select Tier',
            description:
              t('common:guide_ocr_step2_desc') ??
              'Pick an OCR model tier based on accuracy and speed. Larger tiers are more accurate but slower.',
          },
          {
            icon: Import,
            title: t('common:guide_ocr_step3_title') ?? 'Extract & Import',
            description:
              t('common:guide_ocr_step3_desc') ??
              'Review the recognized text and import it as a new object. You can also copy or edit the result.',
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_ocr_scan') ?? 'OCR & Scan',
            description:
              t('common:guide_help_ocr_scan_desc') ??
              'Scan images and import recognized text as objects',
            href: '/help?id=ocr_scan',
          },
        ],
      },
    ],
    [t],
  );

  /** 切换扫描模式并清空既有结果（ScanDropZone 回调，保持原内联清理语义）。 */
  const handleScanModeChange = (mode: ScanMode) => {
    setScanMode(mode);
    setResult(null);
    setMrzResult(null);
  };
  return (
    <AppShell
      title={t('ocr:title')}
      onBack={() => {
        const state = location.state as { fromHome?: boolean } | undefined;
        if (state?.fromHome) {
          navigate('/home');
        } else {
          navigate(-1);
        }
      }}
      actions={<PageGuideButton pages={ocrGuidePages} />}
    >
      <PageContainer variant="wide" gap="default">
        <OcrScanSettingsPanel
          isMobilePlatform={isMobilePlatform}
          tiers={tiers}
          activeTier={activeTier}
          statusMap={statusMap}
          loadingStatus={loadingStatus}
          installingTier={installingTier}
          downloadingTier={downloadingTier}
          downloadUrl={downloadUrl}
          onDownloadUrlChange={setDownloadUrl}
          onTierChange={handleTierChange}
          onInstallBundled={handleInstallBundled}
          onDownload={handleDownload}
        />

        <ScanDropZone
          scanMode={scanMode}
          onScanModeChange={handleScanModeChange}
          isScanning={isScanning}
          isMobilePlatform={isMobilePlatform}
          activeTier={activeTier}
          onSelectFile={handleSelectFile}
          onTakePhoto={handleTakePhoto}
        />

        <OcrResultList
          result={result}
          mrzResult={mrzResult}
          isScanning={isScanning}
          isImporting={isImporting}
          isNameDialogOpen={isNameDialogOpen}
          importNameDefault={importNameDefault}
          onImportAsObject={handleImportAsObject}
          onConfirmImport={handleConfirmImport}
          onCancelImport={() => {
            setIsNameDialogOpen(false);
            setPendingImportSource(null);
          }}
        />
      </PageContainer>
    </AppShell>
  );
}
