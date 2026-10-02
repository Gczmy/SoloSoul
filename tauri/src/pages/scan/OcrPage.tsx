import { useEffect, useState, useMemo, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useEntryBack } from '@/hooks/useEntryBack';
import { useTranslation } from 'react-i18next';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useToastError } from '@/hooks/useToastError';
import { useOcrModelManager } from '@/hooks/useOcrModelManager';
import { supportsOcrScanSync } from '@/lib/ocrCapabilities';
import { isMobilePlatformSync } from '@/lib/platform';

import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import {
  createOcrScanOperation,
  type OcrJobState,
  type OcrScanOperation,
} from '@/lib/ocrScanOperation';
import { translateOcrError } from '@/lib/ocrScanMessages';
import type { MrzResult, OcrResult } from '@/lib/ipc';
import { Info, Import, Layers, Scan } from 'lucide-react';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { OcrResultList } from './OcrResultList';
import { OcrScanSettingsPanel } from './OcrScanSettingsPanel';
import { ScanDropZone, type ScanMode } from './ScanDropZone';

export function OcrPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const handleBack = useEntryBack(() => {
    const state = location.state as { fromHome?: boolean } | undefined;
    if (state?.fromHome) navigate('/');
    else navigate(-1);
  });
  const { t } = useTranslation(['ocr', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  // P047: 仅订阅 createObject action（store 级选择器，避免任意字段变化触发整页重渲染）
  const createObject = useObjectStore((s) => s.createObject);
  const { onError, onSuccess } = useToastError();

  const initialFilePath = (location.state as { filePath?: string } | null)?.filePath || '';

  const [result, setResult] = useState<OcrResult | null>(null);
  const [mrzResult, setMrzResult] = useState<MrzResult | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [scanState, setScanState] = useState<OcrJobState | null>(null);
  const requests = useRef(createSessionRequests());
  const operationRef = useRef<OcrScanOperation | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [isNameDialogOpen, setIsNameDialogOpen] = useState(false);
  const [importNameDefault, setImportNameDefault] = useState('');
  const [pendingImportSource, setPendingImportSource] = useState<'ocr' | 'mrz' | null>(null);
  const [scanMode, setScanMode] = useState<ScanMode>('general');

  const isMobilePlatform = isMobilePlatformSync();
  const scanSupported = supportsOcrScanSync();

  const handleScanError = (error: unknown) => {
    onError(new Error(translateOcrError(error, t)), t('ocr:scan_failed'));
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
    enabled: scanSupported,
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

  useEffect(() => {
    const invalidate = () => {
      operationRef.current?.dispose();
      operationRef.current = null;
      requests.current.invalidate();
    };
    const unsubscribe = onRequestSessionChange(() => {
      invalidate();
      setResult(null);
      setMrzResult(null);
      setIsScanning(false);
      setScanState(null);
      setIsImporting(false);
      setIsNameDialogOpen(false);
      setPendingImportSource(null);
    });
    return () => {
      invalidate();
      unsubscribe();
    };
  }, []);

  const performScan = async (path: string) => {
    if (!scanSupported) return;
    if (!guardActiveModelInstalled()) return;
    operationRef.current?.dispose();
    const ticket = requests.current.begin('scan', accountId);
    setIsScanning(true);
    setScanState('queued');
    setResult(null);
    setMrzResult(null);
    const operation = createOcrScanOperation({
      accountId,
      isCurrent: ticket.isCurrent,
      onState: setScanState,
    });
    operationRef.current = operation;
    const outcome = await operation.run(path, scanMode);
    if (!ticket.isCurrent() || !operation.isCurrent() || operationRef.current !== operation) return;
    operationRef.current = null;
    setIsScanning(false);
    setScanState(outcome.status);
    if (outcome.status === 'completed') {
      setResult(outcome.result);
      setMrzResult(outcome.mrzResult);
      if (outcome.usedFallback) onSuccess(t('ocr:mrz_no_detected'));
    } else if (outcome.status === 'failed') {
      handleScanError(outcome.error);
    }
  };

  // 附件自动扫描与手动入口使用同一操作；路径变更或卸载使旧操作失效。
  useEffect(() => {
    if (!initialFilePath) return;
    const scanRequests = requests.current;
    void performScan(initialFilePath);
    return () => {
      operationRef.current?.dispose();
      operationRef.current = null;
      scanRequests.invalidate('scan');
    };
    // 只对传入路径启动一次，不因模型状态或渲染重新发起。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialFilePath]);

  const handleSelectFile = async () => {
    if (!scanSupported) return;
    const ticket = requests.current.begin('picker', accountId);
    try {
      const { openWithPause } = await import('@/lib/dialog');
      if (!ticket.isCurrent()) return;
      const path = await openWithPause({
        filters: getFileFilters(),
        multiple: false,
        title:
          scanMode === 'mrz' || isMobilePlatform || activeTier === 'vision'
            ? t('ocr:select_image_title')
            : t('ocr:select_file_title'),
      });
      if (ticket.isCurrent() && path && typeof path === 'string') await performScan(path);
    } catch (error) {
      if (ticket.isCurrent()) onError(error, t('ocr:select_image_failed'));
    }
  };

  const handleTakePhoto = async () => {
    if (!scanSupported) return;
    const ticket = requests.current.begin('picker', accountId);
    setResult(null);
    setMrzResult(null);
    setIsScanning(true);
    setScanState(null);
    try {
      const { useAutoLockPauseStore } = await import('@/stores/autoLockPauseStore');
      if (!ticket.isCurrent()) return;
      const { pause, resume } = useAutoLockPauseStore.getState();
      pause();
      try {
        const path = await ticket.invoke<string | null>('mobile_ocr_take_photo');
        if (!ticket.isCurrent()) return;
        if (path) await performScan(path);
        else {
          setIsScanning(false);
          onError(t('ocr:take_photo_no_image'), t('ocr:take_photo_failed'));
        }
      } finally {
        resume();
      }
    } catch (error) {
      if (ticket.isCurrent()) {
        setIsScanning(false);
        onError(error, t('ocr:take_photo_failed'));
      }
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
    const ticket = requests.current.begin('import', accountId);
    setIsNameDialogOpen(false);
    setIsImporting(true);
    try {
      await createObject({
        accountId,
        name,
        typeId: 'document',
        properties: buildImportProperties(),
      });
      if (ticket.isCurrent()) onSuccess(t('ocr:import_success'));
    } catch (e) {
      if (ticket.isCurrent()) onError(e, t('ocr:import_failed'));
    } finally {
      if (ticket.isCurrent()) {
        setIsImporting(false);
        setPendingImportSource(null);
      }
    }
  };

  const ocrGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_ocr_title', { defaultValue: 'OCR Scan Guide' }),
        steps: [
          {
            icon: Scan,
            title: t('common:guide_ocr_step1_title', { defaultValue: 'Choose Input' }),
            description: t('common:guide_ocr_step1_desc', {
              defaultValue:
                'Take a photo with the camera or select an image from your device. OCR extracts text from the image.',
            }),
          },
          {
            icon: Layers,
            title: t('common:guide_ocr_step2_title', { defaultValue: 'Select Tier' }),
            description: t('common:guide_ocr_step2_desc', {
              defaultValue:
                'Pick an OCR model tier based on accuracy and speed. Larger tiers are more accurate but slower.',
            }),
          },
          {
            icon: Import,
            title: t('common:guide_ocr_step3_title', { defaultValue: 'Extract & Import' }),
            description: t('common:guide_ocr_step3_desc', {
              defaultValue:
                'Review the recognized text and import it as a new object. You can also copy or edit the result.',
            }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_ocr_scan', { defaultValue: 'OCR & Scan' }),
            description: t('common:guide_help_ocr_scan_desc', {
              defaultValue: 'Scan images and import recognized text as objects',
            }),
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
    <PageShell
      title={t('ocr:title')}
      onBack={handleBack}
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
          scanSupported={scanSupported}
          scanMode={scanMode}
          onScanModeChange={handleScanModeChange}
          isScanning={isScanning}
          isMobilePlatform={isMobilePlatform}
          activeTier={activeTier}
          scanState={scanState}
          onCancel={() => operationRef.current?.cancel()}
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
    </PageShell>
  );
}
