import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Copy, Check, Eye, Download } from 'lucide-react';
import { open } from '@tauri-apps/plugin-shell';
import { save, open as openDialog } from '@tauri-apps/plugin-dialog';
import { copyFile } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { Button } from '@/components/ui/Button';
import styles from './PluginResultPanel.module.css';
import type { PluginResultPayload, WatermarkResultItem } from '@/lib/plugin';
import { ICON_SIZE } from '@/lib/iconSizes';


// ─── 国家名称 → ISO 3166-1 alpha-2 代码映射 ───────────────────────────────
const COUNTRY_NAME_TO_CODE: Record<string, string> = {
  'United Kingdom': 'GB',
  'Great Britain': 'GB',
  'England': 'GB',
  'United States': 'US',
  'USA': 'US',
  'America': 'US',
  'China': 'CN',
  'Japan': 'JP',
  'Germany': 'DE',
  'France': 'FR',
  'Italy': 'IT',
  'Spain': 'ES',
  'Canada': 'CA',
  'Australia': 'AU',
  'Brazil': 'BR',
  'India': 'IN',
  'Russia': 'RU',
  'South Korea': 'KR',
  'Korea': 'KR',
  'Netherlands': 'NL',
  'Switzerland': 'CH',
  'Sweden': 'SE',
  'Norway': 'NO',
  'Denmark': 'DK',
  'Finland': 'FI',
  'Singapore': 'SG',
  'Hong Kong': 'HK',
  'Taiwan': 'TW',
  'Mexico': 'MX',
  'Argentina': 'AR',
  'New Zealand': 'NZ',
  'Ireland': 'IE',
  'Poland': 'PL',
  'Portugal': 'PT',
  'Belgium': 'BE',
  'Austria': 'AT',
  'Turkey': 'TR',
  'Thailand': 'TH',
  'Vietnam': 'VN',
  'Malaysia': 'MY',
  'Indonesia': 'ID',
  'Philippines': 'PH',
  'South Africa': 'ZA',
  'Egypt': 'EG',
  'Nigeria': 'NG',
  'Kenya': 'KE',
  'Ukraine': 'UA',
  'Czech Republic': 'CZ',
  'Greece': 'GR',
  'Hungary': 'HU',
  'Romania': 'RO',
  'Israel': 'IL',
  'UAE': 'AE',
  'Saudi Arabia': 'SA',
  'Colombia': 'CO',
  'Chile': 'CL',
  'Peru': 'PE',
};

// ─── ISO 代码 → 多语言短标签 ──────────────────────────────────────────────
const COUNTRY_CODE_TO_LABEL: Record<string, { zh: string; en: string }> = {
  GB: { zh: '英国', en: 'UK' },
  US: { zh: '美国', en: 'US' },
  CN: { zh: '中国', en: 'CN' },
  JP: { zh: '日本', en: 'JP' },
  DE: { zh: '德国', en: 'DE' },
  FR: { zh: '法国', en: 'FR' },
  IT: { zh: '意大利', en: 'IT' },
  ES: { zh: '西班牙', en: 'ES' },
  CA: { zh: '加拿大', en: 'CA' },
  AU: { zh: '澳大利亚', en: 'AU' },
  BR: { zh: '巴西', en: 'BR' },
  IN: { zh: '印度', en: 'IN' },
  RU: { zh: '俄罗斯', en: 'RU' },
  KR: { zh: '韩国', en: 'KR' },
  NL: { zh: '荷兰', en: 'NL' },
  CH: { zh: '瑞士', en: 'CH' },
  SE: { zh: '瑞典', en: 'SE' },
  NO: { zh: '挪威', en: 'NO' },
  DK: { zh: '丹麦', en: 'DK' },
  FI: { zh: '芬兰', en: 'FI' },
  SG: { zh: '新加坡', en: 'SG' },
  HK: { zh: '香港', en: 'HK' },
  TW: { zh: '台湾', en: 'TW' },
  MX: { zh: '墨西哥', en: 'MX' },
  AR: { zh: '阿根廷', en: 'AR' },
  NZ: { zh: '新西兰', en: 'NZ' },
  IE: { zh: '爱尔兰', en: 'IE' },
  PL: { zh: '波兰', en: 'PL' },
  PT: { zh: '葡萄牙', en: 'PT' },
  BE: { zh: '比利时', en: 'BE' },
  AT: { zh: '奥地利', en: 'AT' },
  TR: { zh: '土耳其', en: 'TR' },
  TH: { zh: '泰国', en: 'TH' },
  VN: { zh: '越南', en: 'VN' },
  MY: { zh: '马来西亚', en: 'MY' },
  ID: { zh: '印度尼西亚', en: 'ID' },
  PH: { zh: '菲律宾', en: 'PH' },
  ZA: { zh: '南非', en: 'ZA' },
  EG: { zh: '埃及', en: 'EG' },
  NG: { zh: '尼日利亚', en: 'NG' },
  KE: { zh: '肯尼亚', en: 'KE' },
  UA: { zh: '乌克兰', en: 'UA' },
  CZ: { zh: '捷克', en: 'CZ' },
  GR: { zh: '希腊', en: 'GR' },
  HU: { zh: '匈牙利', en: 'HU' },
  RO: { zh: '罗马尼亚', en: 'RO' },
  IL: { zh: '以色列', en: 'IL' },
  AE: { zh: '阿联酋', en: 'UAE' },
  SA: { zh: '沙特', en: 'SA' },
  CO: { zh: '哥伦比亚', en: 'CO' },
  CL: { zh: '智利', en: 'CL' },
  PE: { zh: '秘鲁', en: 'PE' },
};

/** 根据 tag/tagCode 解析出本地化短标签，无匹配时返回默认标签 */
function resolveCountryLabel(
  tag: string | undefined,
  tagCode: string | undefined,
  locale: 'zh' | 'en',
): string {
  // 1. 优先通过 tagCode 查表
  if (tagCode) {
    const upper = tagCode.toUpperCase();
    // 插件未识别到国家时返回 DEFAULT，需要国际化为“默认/Default”
    if (upper === 'DEFAULT') {
      return locale === 'zh' ? '默认' : 'Default';
    }
    const entry = COUNTRY_CODE_TO_LABEL[upper];
    if (entry) return locale === 'zh' ? entry.zh : entry.en;
    return upper; // 未知代码，直接显示大写代码
  }

  // 2. 通过 tag（英文国家名）查代码
  if (tag) {
    const code = COUNTRY_NAME_TO_CODE[tag];
    if (code) {
      const entry = COUNTRY_CODE_TO_LABEL[code];
      if (entry) return locale === 'zh' ? entry.zh : entry.en;
      return code;
    }
    // tag 存在但未识别 → 通用标签
    return locale === 'zh' ? '通用' : 'Any';
  }

  // 3. 无 tag 也无 tagCode → 显示默认标签
  return locale === 'zh' ? '默认' : 'Default';
}

interface PluginResultPanelProps {
  results: PluginResultPayload[];
}

export function PluginResultPanel({ results }: PluginResultPanelProps) {
  const { t } = useTranslation('plugin');

  if (results.length === 0) {
    return (
      <div className={styles.empty}>{t('result_empty', { defaultValue: 'No result yet' })}</div>
    );
  }

  return (
    <div className={styles.container}>
      {results.map((result, index) => (
        <div key={index} className={styles.resultCard}>
          <ResultContent payload={result} />
        </div>
      ))}
    </div>
  );
}

function PerPairCopyRow({
  pair,
}: {
  pair: { key: string; value: string; tag?: string; tagCode?: string };
}) {
  const { t, i18n } = useTranslation('plugin');
  const [copied, setCopied] = useState(false);
  const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';

  const copyPair = async () => {
    try {
      await navigator.clipboard.writeText(`${pair.key}: ${pair.value}`);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // 静默忽略
    }
  };

  const badgeLabel = resolveCountryLabel(pair.tag, pair.tagCode, locale);

  return (
    <div className={styles.pairRow}>
      <div className={styles.pairRowHeader}>
        {badgeLabel && (
          <span className={styles.countryBadge} title={pair.tag || pair.tagCode || ''}>
            {badgeLabel}
          </span>
        )}
        <span className={styles.pairKey} title={pair.key}>{pair.key}</span>
        <button
          type="button"
          className={`${styles.pairCopyBtn} ${copied ? styles.pairCopyBtnActive : ''}`}
          onClick={copyPair}
          title={t('copy_entry', { defaultValue: 'Copy this entry' })}
          aria-label={t('copy_entry', { defaultValue: 'Copy this entry' })}
        >
          {copied ? <Check size={ICON_SIZE.xs} /> : <Copy size={ICON_SIZE.xs} />}
        </button>
      </div>
      <span className={styles.pairValue}>{pair.value}</span>
    </div>
  );
}

function ResultContent({ payload }: { payload: PluginResultPayload }) {
  const { t } = useTranslation('plugin');

  switch (payload.type) {
    case 'text':
      return <p className={styles.text}>{payload.content}</p>;

    case 'key_value':
      return (
        <div className={styles.keyValueList}>
          
          {payload.pairs.map((pair, idx) => (
            <PerPairCopyRow key={idx} pair={pair} />
          ))}
        </div>
      );

    case 'table':
      return (
        <div className={styles.tableWrapper}>
          <table className={styles.table}>
            <thead>
              <tr>
                {payload.headers.map((h, i) => (
                  <th key={i}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {payload.rows.map((row, i) => (
                <tr key={i}>
                  {row.map((cell, j) => (
                    <td key={j}>{cell}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );

    case 'markdown':
      return <pre className={styles.markdown}>{payload.content}</pre>;

    case 'watermark_result':
      return <WatermarkResultContent payload={payload} />;

    default:
      return (
        <div className={styles.unknown}>
          {t('result_unknown', { defaultValue: 'Unsupported result type' })}
        </div>
      );
  }
}

function WatermarkResultContent({ payload }: { payload: PluginResultPayload & { type: 'watermark_result' } }) {
  const { t } = useTranslation('plugin');
  const items = payload.items;

  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const resultItemId = useCallback((item: WatermarkResultItem) => {
    return `${item.objectId}-${item.attachmentId}`;
  }, []);

  const allSelected = items.length > 0 && items.every((item) => selectedIds.has(resultItemId(item)));
  const someSelected = selectedIds.size > 0 && !allSelected;

  const handleToggle = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleSelectAll = () => {
    if (allSelected) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(items.map(resultItemId)));
    }
  };

  const handlePreview = async (path: string) => {
    try {
      const fileUrl = new URL(path.replace(/\\/g, '/'), 'file://').href;
      await open(fileUrl);
    } catch {
      // silent in sidebar
    }
  };

  const handleDownload = async (item: WatermarkResultItem) => {
    try {
      const dest = await save({ defaultPath: item.fileName });
      if (dest) {
        await copyFile(item.outputPath, dest);
      }
    } catch {
      // silent
    }
  };

  const handleDownloadSelected = async () => {
    if (selectedIds.size === 0) return;
    try {
      const dir = await openDialog({ directory: true });
      if (!dir) return;
      const selected = items.filter((item) => selectedIds.has(resultItemId(item)));
      await Promise.all(
        selected.map(async (item) => {
          const dest = await join(dir, item.fileName);
          await copyFile(item.outputPath, dest);
        }),
      );
    } catch {
      // silent
    }
  };

  return (
    <div className={styles.watermarkResult}>
      {/* 全选 + 批量操作（同一行） */}
      <div className={styles.watermarkSelectAll}>
        <div className={styles.watermarkSelectAllLeft} onClick={handleSelectAll}>
          <SelectCheckbox
            checked={allSelected}
            indeterminate={someSelected}
          />
          <span className={styles.watermarkSelectAllLabel}>
            {t('watermark.select_all', { defaultValue: '全选' })}
          </span>
          <span className={styles.watermarkSelectedCount}>
            {t('watermark.selected_count', {
              defaultValue: '已选 {{count}} 项',
              count: selectedIds.size,
            })}
          </span>
        </div>
        {selectedIds.size > 0 && (
          <div className={styles.watermarkSelectAllRight}>
            <Button variant="secondary" size="sm" onClick={handleDownloadSelected}>
              <Download size={ICON_SIZE.xs} />
              {t('watermark.download_selected', {
                defaultValue: '下载已选项 ({{count}})',
                count: selectedIds.size,
              })}
            </Button>
            <button
              className={styles.watermarkClearSelection}
              onClick={() => setSelectedIds(new Set())}
            >
              {t('common:clear_selection', { defaultValue: '清除选择' })}
            </button>
          </div>
        )}
      </div>

      {/* 文件列表 */}
      <div className={styles.watermarkList}>
        {items.map((item) => (
          <div key={resultItemId(item)} className={styles.watermarkItem}>
            <div
              className={styles.watermarkMain}
              onClick={() => handleToggle(resultItemId(item))}
            >
              <SelectCheckbox checked={selectedIds.has(resultItemId(item))} />
              <div className={styles.watermarkInfo}>
                <span className={styles.watermarkName} title={item.fileName}>
                  {item.fileName}
                </span>
                <span className={styles.watermarkMime}>{item.mimeType}</span>
              </div>
            </div>
            <div className={styles.watermarkActions}>
              <BadgeIconButton
                Icon={Eye}
                onClick={() => handlePreview(item.outputPath)}
                title={t('watermark.preview', { defaultValue: '预览' })}
              />
              <BadgeIconButton
                Icon={Download}
                onClick={() => handleDownload(item)}
                title={t('watermark.download', { defaultValue: '下载' })}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

