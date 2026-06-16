import { useState, useCallback, useRef, useMemo, useEffect } from 'react';
import { Search, X } from 'lucide-react';
import { Input } from '@/components/ui/Input';
import { Card } from '@/components/ui/Card';
import type { GuideContent } from '@/lib/guideApi';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';

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
      setLoading(true);
      try {
        const res = await onSearch(q.trim());
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
      <div style={{ position: 'relative' }}>
        <Search
          size={18}
          style={{
            position: 'absolute',
            left: 12,
            top: '50%',
            transform: 'translateY(-50%)',
            color: 'var(--text-tertiary)',
          }}
        />
        <Input
          placeholder="搜索帮助文档..."
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          style={{ paddingLeft: 38 }}
        />
        {query && (
          <button
            onClick={() => {
              setQuery('');
              setResults(null);
            }}
            style={{
              position: 'absolute',
              right: 10,
              top: '50%',
              transform: 'translateY(-50%)',
              background: 'none',
              border: 'none',
              color: 'var(--text-tertiary)',
              cursor: 'pointer',
              padding: 4,
            }}
          >
            <X size={16} />
          </button>
        )}
      </div>

      {loading && (
        <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', fontSize: 13 }}>
          搜索中...
        </p>
      )}

      {!loading && results !== null && results.length === 0 && (
        <p style={{ textAlign: 'center', color: 'var(--text-secondary)', fontSize: 14 }}>
          未找到匹配的文档
        </p>
      )}

      {!loading && results && results.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {results.map((r) => {
            const excerpt = r.content
              .slice(0, 260)
              .replace(/[#*`[\]!]/g, '')
              .replace(/\n+/g, ' ');
            return (
              <Card key={r.id} interactive onClick={() => onSelect(r.id)}>
                <div style={{ fontSize: 14, fontWeight: 500, marginBottom: 4 }}>
                  <HighlightText text={r.title} tokens={tokens} />
                </div>
                <div
                  style={{
                    fontSize: 12,
                    color: 'var(--text-secondary)',
                    lineHeight: 1.5,
                    display: '-webkit-box',
                    WebkitLineClamp: 2,
                    WebkitBoxOrient: 'vertical',
                    overflow: 'hidden',
                  }}
                >
                  <HighlightText text={excerpt} tokens={tokens} />
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
