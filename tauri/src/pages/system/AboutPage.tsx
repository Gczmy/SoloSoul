import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { Code, Info, Shield } from 'lucide-react';
import { useUpdateChecker, type AppInfo, type VersionInfo } from '@/hooks/useUpdateChecker';
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
  const docLang = i18n.language?.startsWith('zh') ? 'zh-CN' : 'en-US';
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

  // ==================== DEBUG-START（临时调试代码，验证后删除）====================
  // 强制展示「关键安全更新」全屏卡片（MandatoryUpdateOverlay）供 UI 修复验证：
  // 伪造 isMandatory=true 并注入示例版本信息/发布说明，打开「关于」页即直接显示。
  // 删除方法：删除本 DEBUG 块，并把下方 MandatoryUpdateOverlay 的
  // isMandatory/info/versionInfo 三处传参恢复为 isMandatory/info/versionInfo。
  const DEBUG_FORCE_MANDATORY_OVERLAY = true;
  const debugInfo: AppInfo | null = info ?? {
    appName: 'SoloSoul',
    version: '2.7.1',
    os: 'macOS',
    arch: 'arm64',
  };
  const debugVersionInfo: VersionInfo | null = {
    ...(versionInfo ?? {}),
    currentVersion: debugInfo?.version ?? '',
    latestVersion: '2.7.2',
    state: 'available',
    mandatory: true,
    body:
      versionInfo?.body ??
      '- 修复若干安全问题\n- 增强数据加密强度\n- 性能优化与稳定性提升',
  };
  // ====================== DEBUG-END ======================

  const links = [
    {
      labelKey: 'github_repo',
      url: 'https://github.com/Gczmy/SoloSoul',
      icon: <Code size={ICON_SIZE.sm} />,
    },
    {
      labelKey: 'privacy_policy',
      url: `https://github.com/Gczmy/SoloSoul/blob/main/docs/${docLang}/PRIVACY_POLICY.md`,
      icon: <Shield size={ICON_SIZE.sm} />,
    },
    {
      labelKey: 'terms_of_service',
      url: `https://github.com/Gczmy/SoloSoul/blob/main/docs/${docLang}/TERMS_OF_SERVICE.md`,
      icon: <Info size={ICON_SIZE.sm} />,
    },
  ];

  return (
    <>
      {/* 关于页面更新说明 Markdown 样式 */}
      <style>{`
        .release-notes-md h1,
        .release-notes-md h2,
        .release-notes-md h3 {
          font-size: var(--text-sm);
          font-weight: 600;
          margin: 8px 0 4px;
        }
        .release-notes-md h4,
        .release-notes-md h5,
        .release-notes-md h6 {
          font-size: var(--text-caption);
          font-weight: 600;
          margin: 6px 0 3px;
        }
        .release-notes-md p { margin: 0 0 6px; }
        .release-notes-md p:last-child { margin-bottom: 0; }
        .release-notes-md ul,
        .release-notes-md ol { margin: 4px 0; padding-left: 20px; }
        .release-notes-md li { margin: 2px 0; }
        .release-notes-md strong { font-weight: 600; }
        .release-notes-md code {
          font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
          font-size: 12px;
          background: rgba(128,128,128,0.1);
          padding: 1px 4px;
          border-radius: 3px;
        }
        .release-notes-md pre {
          background: var(--bg-toolbar);
          padding: 8px 10px;
          border-radius: 6px;
          overflow-x: auto;
          margin: 6px 0;
        }
        .release-notes-md pre code {
          background: none;
          padding: 0;
        }
        .release-notes-md a {
          color: var(--accent-primary);
          text-decoration: none;
        }
        @media (hover: hover) and (pointer: fine) {
          .release-notes-md a:hover { text-decoration: underline; }
        }
        .release-notes-md blockquote {
          border-left: 3px solid var(--accent-primary);
          margin: 6px 0;
          padding-left: 10px;
          color: var(--text-secondary);
        }
        .release-notes-md hr {
          border: none;
          border-top: 1px solid var(--border-subtle);
          margin: 8px 0;
        }
        @keyframes about-retry-spin {
          to { transform: rotate(360deg); }
        }
        .about-retry-spin { animation: about-retry-spin 1s linear infinite; }
      `}</style>
      <AppShell
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
          isMandatory={DEBUG_FORCE_MANDATORY_OVERLAY || isMandatory}
          info={debugInfo}
          versionInfo={debugVersionInfo}
          downloading={downloading}
          downloadedBytes={downloadedBytes}
          totalBytes={totalBytes}
          progressPercent={progressPercent}
          downloadError={downloadError}
          handleUpdate={handleUpdate}
        />
      </AppShell>
    </>
  );
}
