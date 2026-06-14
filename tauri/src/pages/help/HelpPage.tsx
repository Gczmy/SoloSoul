import { useState, useEffect, useCallback } from 'react';
import { useNavigate, useSearchParams, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useCancellable } from '@/hooks/useCancellable';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { GuideRenderer } from '@/components/guide/GuideRenderer';
import { GuideIndex } from '@/components/guide/GuideIndex';
import { GuideSearch } from '@/components/guide/GuideSearch';
import { OnboardingDialog } from '@/components/onboarding/OnboardingDialog';
import {
  loadGuideIndex,
  loadGuideContent,
  searchGuides,
  type GuideIndex as GuideIndexType,
  type GuideContent,
} from '@/lib/guideApi';
import { BookOpen, RefreshCw, Loader2 } from 'lucide-react';

export function HelpPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const guideId = searchParams.get('id') || '';
  const backTo = (location.state as { from?: string } | null)?.from;
  const { t, i18n } = useTranslation(['common', 'settings']);
  const language = i18n.language || 'zh-CN';
  const makeCancellable = useCancellable();

  const [index, setIndex] = useState<GuideIndexType | null>(null);
  const [content, setContent] = useState<GuideContent | null>(null);
  const [loading, setLoading] = useState(true);
  const [indexLoading, setIndexLoading] = useState(true);
  const [error, setError] = useState<{ title: string; message: string; isTimeout: boolean } | null>(
    null,
  );
  const [showTutorial, setShowTutorial] = useState(false);

  const formatIndexError = (e: unknown) => {
    const msg = e instanceof Error ? e.message : String(e);
    const isTimeout = msg.includes('超时') || msg.includes('timeout');
    const isNoCommand =
      msg.includes('not found') || msg.includes('未找到') || msg.includes('no such command');

    if (isTimeout || isNoCommand) {
      return {
        title: 'Tauri 后端未响应',
        message:
          '帮助文档需要本地 Tauri 后端支持。可能的解决方式：\n' +
          '1. 确认你是在 tauri 目录下运行 npm run tauri dev\n' +
          '2. 检查终端是否显示 Rust 编译错误或 panic\n' +
          '3. 首次启动 Rust 编译可能需要 30–90 秒，请等待窗口完全加载\n' +
          '4. 刷新页面重试\n\n原始错误：' +
          msg,
        isTimeout: true,
      };
    }

    return {
      title: '无法加载帮助索引',
      message: msg,
      isTimeout: false,
    };
  };

  const loadIndex = useCallback(() => {
    const { isCancelled } = makeCancellable();
    setIndexLoading(true);
    setError(null);
    loadGuideIndex()
      .then((idx) => {
        if (!isCancelled()) setIndex(idx);
      })
      .catch((e) => {
        if (!isCancelled()) setError(formatIndexError(e));
      })
      .finally(() => {
        if (!isCancelled()) setIndexLoading(false);
      });
  }, [makeCancellable]);

  const loadContent = useCallback(
    (id: string) => {
      const { isCancelled } = makeCancellable();
      if (!id) {
        if (!isCancelled()) setContent(null);
        return;
      }
      setLoading(true);
      loadGuideContent(id, language)
        .then((c) => {
          if (!isCancelled()) {
            setContent(c);
            setError(null);
          }
        })
        .catch((e) => {
          if (!isCancelled()) {
            const msg = e instanceof Error ? e.message : String(e);
            setError({ title: '无法加载文档内容', message: msg, isTimeout: false });
          }
        })
        .finally(() => {
          if (!isCancelled()) setLoading(false);
        });
    },
    [language, makeCancellable],
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
    if (guideId) {
      navigate('/help', { replace: true });
    } else if (backTo) {
      navigate(backTo, { replace: true });
    } else {
      navigate('/home', { replace: true });
    }
  };

  const handleSearch = async (query: string): Promise<GuideContent[]> => {
    return searchGuides(query, language);
  };

  return (
    <AppShell
      title={content ? content.title : '帮助文档'}
      onBack={handleBack}
      actions={
        guideId ? undefined : <BookOpen size={20} style={{ color: 'var(--text-secondary)' }} />
      }
    >
      <div
        style={{
          maxWidth: 720,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 20,
        }}
      >
        {error && (
          <div
            style={{
              padding: '16px 20px',
              borderRadius: 10,
              background: 'var(--color-error-bg)',
              border: '1px solid var(--color-error-border)',
              color: 'var(--color-error-text)',
              fontSize: 14,
            }}
          >
            <div style={{ fontWeight: 600, marginBottom: 8 }}>{error.title}</div>
            <div style={{ whiteSpace: 'pre-line', lineHeight: 1.6 }}>{error.message}</div>
            <button
              onClick={loadIndex}
              style={{
                marginTop: 12,
                display: 'inline-flex',
                alignItems: 'center',
                gap: 6,
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--color-error-border)',
                background: 'transparent',
                color: 'var(--color-error-text)',
                fontSize: 13,
                cursor: 'pointer',
              }}
            >
              <RefreshCw size={14} />
              重试
            </button>
          </div>
        )}

        {!guideId && indexLoading && (
          <Card>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 12,
                padding: '40px 16px',
                color: 'var(--text-secondary)',
              }}
            >
              <Loader2
                size={24}
                style={{
                  animation: 'spin 1s linear infinite',
                  color: 'var(--accent-primary)',
                }}
              />
              <p style={{ fontSize: 14 }}>{t('common:loading', '正在加载...')}</p>
            </div>
          </Card>
        )}

        {!guideId && !indexLoading && index && (
          <>
            <GuideSearch onSearch={handleSearch} onSelect={handleSelect} />
            <GuideIndex
              guides={index.guides}
              categories={index.categories}
              language={language}
              onSelect={handleSelect}
              extraItems={{
                basics: (
                  <Card interactive onClick={() => setShowTutorial(true)}>
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                      }}
                    >
                      <span style={{ fontSize: 14, fontWeight: 500 }}>
                        {t('common:tutorial')}
                      </span>
                      <span style={{ color: 'var(--text-tertiary)', fontSize: 18 }}>›</span>
                    </div>
                  </Card>
                ),
              }}
            />
          </>
        )}

        {guideId && loading && <LoadingPlaceholder variant="base" minHeight={200} />}

        {guideId && content && !loading && (
          <div>
            <GuideRenderer
              content={content.content}
              onLinkClick={(href) => {
                const guideIdFromHref = href.replace(/\.md$/, '').split('/').pop() || href;
                setSearchParams({ id: guideIdFromHref });
              }}
            />
          </div>
        )}
      </div>
      {showTutorial && (
        <OnboardingDialog
          onComplete={() => setShowTutorial(false)}
          onSkip={() => setShowTutorial(false)}
        />
      )}
    </AppShell>
  );
}
