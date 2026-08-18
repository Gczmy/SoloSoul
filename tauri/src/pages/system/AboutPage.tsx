import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { Code, Info, Shield } from 'lucide-react';
import { useUpdateChecker } from '@/hooks/useUpdateChecker';
import { useUiStore } from '@/stores/uiStore';
import { ICON_SIZE } from '@/lib/constants';
import { UpdateInfoCard } from './UpdateInfoCard';
import { LinksCard } from './LinksCard';
import { LegalFooter } from './LegalFooter';
import { MandatoryUpdateOverlay } from './MandatoryUpdateOverlay';

/**
 * 关于页：
 * - 更新检查/双平台下载安装状态机收敛于 useUpdateChecker hook
 * - 本组件仅负责翻译、导航与视图渲染
 */
export function AboutPage() {
  const navigate = useNavigate();
  const { t, i18n } = useTranslation(['settings', 'common']);
  const showToast = useUiStore((s) => s.showToast);
  // 法律条款文档合并至 docs/legal/，文件名按各自语言命名（中文名/英文名）
  const isZh = i18n.language?.startsWith('zh');
  const legalDoc = (zhName: string, enName: string) =>
    `https://github.com/Gczmy/SoloSoul/blob/main/docs/legal/${isZh ? zhName : enName}.md`;
  const updater = useUpdateChecker();
  const {
    info,
    versionInfo,
    loading,
    checking,
    downloading,
    downloadProgress,
    downloadedBytes,
    totalBytes,
    downloadError,
    progressPercent,
    isMandatory,
    runCheck,
    handleUpdate,
  } = updater;

  const links = [
    {
      labelKey: 'github_repo',
      url: 'https://github.com/Gczmy/SoloSoul',
      icon: <Code size={ICON_SIZE.sm} />,
    },
    {
      labelKey: 'privacy_policy',
      url: legalDoc('隐私政策', 'Privacy Policy'),
      icon: <Shield size={ICON_SIZE.sm} />,
    },
    {
      labelKey: 'terms_of_service',
      url: legalDoc('服务条款', 'Terms of Service'),
      icon: <Info size={ICON_SIZE.sm} />,
    },
  ];

  return (
    <>
      {/* 关于页面重试按钮旋转动画（release-notes-md 全局样式见 global.css） */}
      <style>{`
        @keyframes about-retry-spin {
          to { transform: rotate(360deg); }
        }
        .about-retry-spin { animation: about-retry-spin 1s linear infinite; }
      `}</style>
      <PageShell
        title={t('settings:about')}
        onBack={isMandatory ? undefined : () => navigate('/settings')}
      >
        <PageContainer variant="form" gap="default">
          <div style={{ textAlign: 'center', padding: '20px 0' }}>
            <ShieldLogo
              size={ICON_SIZE['6xl']}
              style={{ margin: '0 auto 14px', boxShadow: '0 4px 16px rgba(0,0,0,0.1)' }}
            />
            <h1
              style={{
                fontSize: 'var(--text-xl)',
                fontWeight: 700,
                margin: 0,
                letterSpacing: '-0.02em',
              }}
            >
              SoloSoul
            </h1>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-tertiary)',
                margin: '6px 0 0',
                maxWidth: 280,
                marginLeft: 'auto',
                marginRight: 'auto',
                lineHeight: 1.5,
              }}
            >
              {t('common:slogan')}
            </p>
          </div>

          <UpdateInfoCard
            loading={loading}
            info={info}
            versionInfo={versionInfo}
            checking={checking}
            downloading={downloading}
            downloadProgress={downloadProgress}
            downloadedBytes={downloadedBytes}
            totalBytes={totalBytes}
            downloadError={downloadError}
            progressPercent={progressPercent}
            runCheck={runCheck}
            handleUpdate={handleUpdate}
          />

          <LinksCard links={links} showToast={showToast} />

          <LegalFooter />
        </PageContainer>

        {/* ── 强制更新全屏覆盖层 ── */}
        <MandatoryUpdateOverlay
          isMandatory={isMandatory}
          info={info}
          versionInfo={versionInfo}
          downloading={downloading}
          downloadedBytes={downloadedBytes}
          totalBytes={totalBytes}
          progressPercent={progressPercent}
          downloadError={downloadError}
          handleUpdate={handleUpdate}
        />
      </PageShell>
    </>
  );
}
