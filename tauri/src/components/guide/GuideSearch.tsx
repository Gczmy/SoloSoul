import { useState, useCallback, useRef, useMemo, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Search } from 'lucide-react';
import { Input } from '@/components/ui/Input';
import { Card } from '@/components/ui/Card';
import type { GuideContent } from '@/lib/guideApi';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { ICON_SIZE } from '@/lib/iconSizes';

/** 帮助文档搜索缓存：同一关键词 30 秒内不重复请求后端 */
const GUIDE_SEARCH_CACHE_TTL = 30_000;
const guideSearchCache = new Map<string, { data: GuideContent[]; timestamp: number }>();


interface GuideSearchProps {
  onSearch: (query: string) => Promise<GuideContent[]>;
  onSelect: (guideId: string) => void;
}

/** 提取搜索 token（简单空格+标点分割，过滤停用词） */
function extractTokens(query: string): string[] {
  const stops = new Set([
    '的',
    '了',
    '是',
    '在',
    '我',
    '有',
    '和',
    '就',
    '不',
    '人',
    '都',
    '一',
    '上',
    '也',
    '很',
    '到',
    '说',
    '要',
    '去',
    '你',
    '会',
    '着',
    '没有',
    '看',
    '好',
    '这',
    'the',
    'a',
    'an',
    'is',
    'are',
    'was',
    'were',
    'be',
    'been',
    'being',
    'have',
    'has',
    'had',
    'do',
    'does',
    'did',
    'will',
    'would',
    'could',
    'should',
    'may',
    'might',
    'must',
    'to',
    'of',
    'in',
    'for',
    'on',
    'with',
    'at',
    'by',
    'from',
    'as',
    'and',
    'but',
    'or',
    'if',
    'i',
    'me',
    'my',
    'you',
    'your',
    'he',
    'him',
    'his',
    'she',
    'her',
    'it',
    'its',
    'they',
    'them',
    'their',
  ]);
  return query
    .toLowerCase()
    .split(/\s+|[^\w\u4e00-\u9fff]+/)
    .filter((t) => t.length >= 2 && !stops.has(t));
}

/**
 * 从文档内容中提取匹配关键词的上下文片段。
 * 找到第一个匹配 token 的位置，取其前后各 contextChars 字符作为摘要。
 */
function extractContextSnippet(content: string, tokens: string[], contextChars = 80): string {
  // 清理 Markdown 标记
  const clean = content
    .replace(/[]#*`[!>|_-]/g, '')
    .replace(/\n{3,}/g, '\n\n')
    .replace(/\s+/g, ' ')
    .trim();

  if (tokens.length === 0) return clean.slice(0, 260);

  // 查找第一个匹配位置（不区分大小写）
  let firstMatch = -1;
  let matchedToken = '';
  const lower = clean.toLowerCase();
  for (const token of tokens) {
    const idx = lower.indexOf(token);
    if (idx !== -1 && (firstMatch === -1 || idx < firstMatch)) {
      firstMatch = idx;
      matchedToken = token;
    }
  }

  if (firstMatch === -1) {
    // 没有匹配到内容（可能只匹配了标题），返回开头
    return clean.slice(0, 260);
  }

  // 取上下文窗口
  const start = Math.max(0, firstMatch - contextChars);
  const end = Math.min(clean.length, firstMatch + matchedToken.length + contextChars);
  let snippet = clean.slice(start, end);

  // 两端加省略号
  if (start > 0) snippet = '…' + snippet;
  if (end < clean.length) snippet = snippet + '…';

  return snippet;
}

/** 高亮文本中的匹配 token */
function HighlightText({ text, tokens }: { text: string; tokens: string[] }) {
  if (tokens.length === 0) return <>{text}</>;

  // 构建正则：匹配任意 token（最长优先）
  const pattern = tokens.map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|');
  const regex = new RegExp(`(${pattern})`, 'gi');
  const parts = text.split(regex);

  return (
    <>
      {parts.map((part, i) => {
        const isMatch = tokens.some((t) => part.toLowerCase() === t);
        if (isMatch) {
          return (
            <mark
              key={`${i}-${tokens.join('-')}`}
              style={{
                background: 'rgba(91, 124, 153, 0.25)',
                color: 'var(--text-primary)',
                borderRadius: 3,
                padding: '0 2px',
                fontWeight: 600,
              }}
            >
              {part}
            </mark>
          );
        }
        return <span key={`${i}-${tokens.join('-')}`}>{part}</span>;
      })}
    </>
  );
}

export function GuideSearch({ onSearch, onSelect }: GuideSearchProps) {
  const { t } = useTranslation('common');
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<GuideContent[] | null>(null);
  const [loading, setLoading] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  const tokens = useMemo(() => extractTokens(query), [query]);

  const doSearch = useCallback(
    async (q: string) => {
      if (!q.trim()) {
        setResults(null);
        return;
      }

      const cacheKey = q.trim().toLowerCase();
      const cached = guideSearchCache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < GUIDE_SEARCH_CACHE_TTL) {
        setResults(cached.data);
        return;
      }

      setLoading(true);
      try {
        const res = await onSearch(q.trim());
        guideSearchCache.set(cacheKey, { data: res, timestamp: Date.now() });
        setResults(res);
      } catch {
        setResults([]);
      } finally {
        setLoading(false);
      }
    },
    [onSearch],
  );

  const handleChange = (val: string) => {
    setQuery(val);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    if (!val.trim()) {
      setResults(null);
      return;
    }
    timeoutRef.current = setTimeout(() => doSearch(val), DEBOUNCE_DELAY_MS);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <Input
        placeholder={t('search_help_docs')}
        value={query}
        onChange={(e) => handleChange(e.target.value)}
        onClear={() => { setQuery(''); setResults(null); }}
        prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
      />

      {loading && (
        <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', fontSize: 'var(--text-body-sm)' }}>
          {t('searching')}
        </p>
      )}

      {!loading && results !== null && results.length === 0 && (
        <p style={{ textAlign: 'center', color: 'var(--text-secondary)', fontSize: 'var(--text-body)' }}>
          {t('no_matching_help_docs')}
        </p>
      )}

      {!loading && results && results.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {results.map((r) => {
            const snippet = extractContextSnippet(r.content, tokens);
            return (
              <Card key={r.id} interactive onClick={() => onSelect(r.id)}>
                <div style={{ fontSize: 'var(--text-body)', fontWeight: 500, marginBottom: 4 }}>
                  <HighlightText text={r.title} tokens={tokens} />
                </div>
                <div
                  style={{
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.5,
                    display: '-webkit-box',
                    WebkitLineClamp: 2,
                    WebkitBoxOrient: 'vertical',
                    overflow: 'hidden',
                  }}
                >
                  <HighlightText text={snippet} tokens={tokens} />
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
