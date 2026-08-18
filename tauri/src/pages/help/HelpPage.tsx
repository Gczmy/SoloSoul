import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate, useSearchParams, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { GuideRenderer, resolveGuideIdFromHref } from '@/components/guide/GuideRenderer';
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
import { motion } from 'framer-motion';
import { BookOpen, RefreshCw } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

export function HelpPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const guideId = searchParams.get('id') || '';
  const backTo = (location.state as { from?: string } | null)?.from;
  const { t, i18n } = useTranslation(['common', 'settings']);
  const language = i18n.language || 'zh-CN';
  const abortIndexRef = useRef<AbortController | null>(null);
  const abortContentRef = useRef<AbortController | null>(null);

  const [index, setIndex] = useState<GuideIndexType | null>(null);
  const [content, setContent] = useState<GuideContent | null>(null);
  const [loading, setLoading] = useState(true);
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
    abortIndexRef.current?.abort();
    const controller = new AbortController();
    abortIndexRef.current = controller;
    setError(null);
    loadGuideIndex()
      .then((idx) => {
        if (!controller.signal.aborted) setIndex(idx);
      })
      .catch((e) => {
        if (!controller.signal.aborted) setError(formatIndexError(e));
      });
  }, []);

  const loadContent = useCallback(
    (id: string) => {
      abortContentRef.current?.abort();
      const controller = new AbortController();
      abortContentRef.current = controller;
      if (!id) {
        if (!controller.signal.aborted) setContent(null);
        return;
      }
      setLoading(true);
      loadGuideContent(id, language)
        .then((c) => {
          if (!controller.signal.aborted) {
            setContent(c);
            setError(null);
          }
        })
        .catch((e) => {
          if (!controller.signal.aborted) {
            const msg = e instanceof Error ? e.message : String(e);
            setError({ title: '无法加载文档内容', message: msg, isTimeout: false });
          }
        })
        .finally(() => {
          if (!controller.signal.aborted) setLoading(false);
        });
    },
    [language],
  );

  useEffect(() => {
    loadIndex();
    return () => abortIndexRef.current?.abort();
  }, [loadIndex]);

  useEffect(() => {
    loadContent(guideId);
    return () => abortContentRef.current?.abort();
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
    <PageShell
      title={content ? content.title : t('settings:items.help_docs')}
      onBack={handleBack}
      actions={
        guideId ? undefined : (
          <BookOpen size={ICON_SIZE.xl} style={{ color: 'var(--text-secondary)' }} />
        )
      }
    >
      <PageContainer variant="wide" gap="default">
        {error && (
          <div
            style={{
              padding: '16px 20px',
              borderRadius: 10,
              background: 'var(--color-error-bg)',
              border: '1px solid var(--color-error-border)',
              color: 'var(--color-error-text)',
              fontSize: 'var(--text-sm)',
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
                fontSize: 'var(--text-body-sm)',
                cursor: 'pointer',
              }}
            >
              <RefreshCw size={ICON_SIZE.sm} />
              重试
            </button>
          </div>
        )}

        {!guideId && index && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-md)' }}
          >
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
                      <span style={{ fontSize: 'var(--text-sm)', fontWeight: 500 }}>
                        {t('common:tutorial')}
                      </span>
                      <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-md)' }}>
                        ›
                      </span>
                    </div>
                  </Card>
                ),
              }}
            />
          </motion.div>
        )}

        {guideId && content && !loading && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
          >
            <GuideRenderer
              content={content.content}
              onLinkClick={(href) => {
                // 文件名可能与 id 不一致（如 device_sync.md → id device-sync），
                // 通过索引反查真实 id，避免 Guide not found。
                setSearchParams({ id: resolveGuideIdFromHref(href, index?.guides) });
              }}
            />
          </motion.div>
        )}
      </PageContainer>
      {showTutorial && (
        <OnboardingDialog
          onComplete={() => setShowTutorial(false)}
          onSkip={() => setShowTutorial(false)}
        />
      )}
    </PageShell>
  );
}
