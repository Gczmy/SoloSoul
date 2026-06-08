import { useState, useEffect, useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { GuideRenderer } from '@/components/guide/GuideRenderer';
import { GuideIndex } from '@/components/guide/GuideIndex';
import { GuideSearch } from '@/components/guide/GuideSearch';
import { loadGuideIndex, loadGuideContent, searchGuides, type GuideIndex as GuideIndexType, type GuideContent } from '@/lib/guideApi';
import { ArrowLeft, BookOpen } from 'lucide-react';

export function HelpPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const guideId = searchParams.get('id') || '';
  const { i18n } = useTranslation(['common', 'settings']);
  const language = i18n.language || 'zh-CN';

  const [index, setIndex] = useState<GuideIndexType | null>(null);
  const [content, setContent] = useState<GuideContent | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadIndex = useCallback(async () => {
    try {
      const idx = await loadGuideIndex();
      setIndex(idx);
      setError(null);
    } catch (e) {
      setError(`无法加载帮助索引: ${e}`);
      console.error('[HelpPage] loadGuideIndex failed:', e);
    }
  }, []);

  const loadContent = useCallback(
    async (id: string) => {
      if (!id) {
        setContent(null);
        return;
      }
      setLoading(true);
      try {
        const c = await loadGuideContent(id, language);
        setContent(c);
        setError(null);
      } catch (e) {
        setError('无法加载文档内容');
      } finally {
        setLoading(false);
      }
    },
    [language]
  );

  useEffect(() => {
    loadIndex();
  }, [loadIndex]);

  useEffect(() => {
    loadContent(guideId);
  }, [guideId, loadContent]);

  const handleSelect = (id: string) => {
    setSearchParams({ id });
  };

  const handleBack = () => {
    setSearchParams({});
  };

  const handleSearch = async (query: string): Promise<GuideContent[]> => {
    return searchGuides(query, language);
  };

  return (
    <AppShell
      title={content ? content.title : '帮助文档'}
      onBack={guideId ? handleBack : undefined}
      actions={
        guideId ? undefined : (
          <BookOpen size={20} style={{ color: 'var(--text-secondary)' }} />
        )
      }
    >
      <div style={{ maxWidth: 720, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 20 }}>
        {error && (
          <div
            style={{
              padding: '16px 20px',
              borderRadius: 10,
              background: 'rgba(231,76,60,0.08)',
              color: '#e74c3c',
              fontSize: 14,
            }}
          >
            {error}
          </div>
        )}

        {!guideId && index && (
          <>
            <GuideSearch onSearch={handleSearch} onSelect={handleSelect} />
            <GuideIndex
              guides={index.guides}
              categories={index.categories}
              language={language}
              onSelect={handleSelect}
            />
          </>
        )}

        {guideId && loading && (
          <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 48 }}>
            加载中...
          </p>
        )}

        {guideId && content && !loading && (
          <div>
            <button
              onClick={handleBack}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                marginBottom: 16,
                background: 'none',
                border: 'none',
                color: 'var(--accent-primary)',
                fontSize: 14,
                cursor: 'pointer',
                padding: 0,
              }}
            >
              <ArrowLeft size={16} />
              返回目录
            </button>
            <h1
              style={{
                fontSize: 24,
                fontWeight: 700,
                marginBottom: 20,
                color: 'var(--text-primary)',
              }}
            >
              {content.title}
            </h1>
            <GuideRenderer content={content.content} />
          </div>
        )}
      </div>
    </AppShell>
  );
}
