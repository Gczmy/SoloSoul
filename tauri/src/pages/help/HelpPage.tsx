import { useState, useEffect, useCallback } from 'react';
import { useNavigate, useSearchParams, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { GuideRenderer } from '@/components/guide/GuideRenderer';
import { GuideIndex } from '@/components/guide/GuideIndex';
import { GuideSearch } from '@/components/guide/GuideSearch';
import { loadGuideIndex, loadGuideContent, searchGuides, type GuideIndex as GuideIndexType, type GuideContent } from '@/lib/guideApi';
import { BookOpen, RefreshCw } from 'lucide-react';

export function HelpPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const guideId = searchParams.get('id') || '';
  const backTo = (location.state as { from?: string } | null)?.from;
  const { i18n } = useTranslation(['common', 'settings']);
  const language = i18n.language || 'zh-CN';

  const [index, setIndex] = useState<GuideIndexType | null>(null);
  const [content, setContent] = useState<GuideContent | null>(null);
  const [loading, setLoading] = useState(true);
  const [indexLoading, setIndexLoading] = useState(true);
  const [indexLoadingElapsed, setIndexLoadingElapsed] = useState(0);
  const [error, setError] = useState<{ title: string; message: string; isTimeout: boolean } | null>(null);

  const formatIndexError = (e: unknown) => {
    const msg = e instanceof Error ? e.message : String(e);
    const isTimeout = msg.includes('超时') || msg.includes('timeout');
    const isNoCommand = msg.includes('not found') || msg.includes('未找到') || msg.includes('no such command');

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

  const loadIndex = useCallback(async () => {
    setIndexLoading(true);
    setIndexLoadingElapsed(0);
    setError(null);
    const timer = setInterval(() => {
      setIndexLoadingElapsed((prev) => prev + 1);
    }, 1000);
    try {
      const idx = await loadGuideIndex();
      setIndex(idx);
    } catch (e) {
      setError(formatIndexError(e));
      // error already surfaced in state
    } finally {
      clearInterval(timer);
      setIndexLoading(false);
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
        const msg = e instanceof Error ? e.message : String(e);
        setError({ title: '无法加载文档内容', message: msg, isTimeout: false });
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
          <div style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 48 }}>
            <p style={{ margin: '0 0 8px' }}>正在连接 Tauri 后端…</p>
            <p style={{ margin: 0, fontSize: 12, opacity: 0.8 }}>
              已等待 {indexLoadingElapsed}s · 首次启动 Rust 编译可能需要 30–120 秒
            </p>
          </div>
        )}

        {!guideId && !indexLoading && index && (
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
    </AppShell>
  );
}
