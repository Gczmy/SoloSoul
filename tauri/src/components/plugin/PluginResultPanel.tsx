import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Copy, Check } from 'lucide-react';
import styles from './PluginResultPanel.module.css';
import type { PluginResultPayload } from '@/lib/plugin';
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

    default:
      return (
        <div className={styles.unknown}>
          {t('result_unknown', { defaultValue: 'Unsupported result type' })}
        </div>
      );
  }
}


