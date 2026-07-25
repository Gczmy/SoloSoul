import { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';

import {
  Sparkles,
  PlusSquare,
  LayoutTemplate,
  ShieldCheck,
  CheckCircle,
  Folder,
  LogIn,
  UserPlus,
} from 'lucide-react';
import { useAuthStore } from '@/stores/authStore';
import type { AccountInfo } from '@/lib/ipc';
import { ICON_SIZE } from '@/lib/constants';
import { getPlatform } from '@/lib/platform';
import { pickVaultDirectory, initVaultDirectory } from '@/lib/vaultDirectory';

interface OnboardingDialogProps {
  onComplete: () => void;
  onSkip: () => void;
}

const baseSteps = [
  { key: 'welcome', icon: Sparkles },
  { key: 'vault_directory', icon: Folder },
  { key: 'create_object', icon: PlusSquare },
  { key: 'templates', icon: LayoutTemplate },
  { key: 'security', icon: ShieldCheck },
  { key: 'finish', icon: CheckCircle },
] as const;

export function OnboardingDialog({ onComplete, onSkip: _onSkip }: OnboardingDialogProps) {
  const { t } = useTranslation('common');
  const [step, setStep] = useState(0);
  const [platformName, setPlatformName] = useState<string>('');
  const [vaultDirActing, setVaultDirActing] = useState(false);
  const [vaultDirError, setVaultDirError] = useState<string | null>(null);
  // 外部目录（SAF）选择后先显示路径，等用户手动点击“下一步”再前进
  const [selectedSafUri, setSelectedSafUri] = useState<string | null>(null);
  // SAF 同步进度阶段：idle（未同步）/ syncing（同步中）/ done（同步完成）
  const [syncPhase, setSyncPhase] = useState<'idle' | 'syncing' | 'done'>('idle');
  const [syncFileName, setSyncFileName] = useState<string>('');
  const [syncFileCount, setSyncFileCount] = useState(0);
  const syncDoneTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const unlistenSync = useRef<{ unregister: () => void } | null>(null);
  // 选择目录后检测到的已有账户列表与决策界面状态
  const [foundAccounts, setFoundAccounts] = useState<AccountInfo[]>([]);
  const [foundAccountCount, setFoundAccountCount] = useState(0);
  const [showAccountDecision, setShowAccountDecision] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    getPlatform().then((p) => {
      setPlatformName(p);
    });
  }, []);

  const isAndroid = platformName === 'android';

  // Wait for platform to load before determining steps, avoiding race where
  // the steps array changes length mid-render (vault_directory step only shown on Android).
  const steps =
    platformName === ''
      ? baseSteps
      : isAndroid
        ? baseSteps
        : baseSteps.filter((s) => s.key !== 'vault_directory');

  const current = steps[step];
  const Icon = current?.icon || Sparkles;
  const isLast = step >= steps.length - 1;

  // 进入 vault_directory 步骤时重置 loading/error 状态，但不重置已选路径，
  // 以便用户返回上一步时仍能看到之前选择的外部目录。
  useEffect(() => {
    if (current?.key === 'vault_directory') {
      setVaultDirError(null);
      setVaultDirActing(false);
      setSyncPhase('idle');
      // 重新进入目录选择步骤时，清除之前的账户决策状态，
      // 让用户重新选择目录/触发同步。
      setShowAccountDecision(false);
      setFoundAccounts([]);
      setFoundAccountCount(0);
    }
    return () => {
      if (syncDoneTimer.current) {
        clearTimeout(syncDoneTimer.current);
        syncDoneTimer.current = null;
      }
      // 清理 Kotlin plugin 事件监听
      if (unlistenSync.current) {
        unlistenSync.current.unregister();
        unlistenSync.current = null;
      }
    };
  }, [current?.key]);

  const handleVaultDirPick = useCallback(async () => {
    // 注意：本回调不直接调用 onComplete，决策卡片由用户选择后再决定
    const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
      (m) => m.useAutoLockPauseStore.getState(),
    );
    pause();
    try {
      setVaultDirActing(true);
      setVaultDirError(null);
      setShowAccountDecision(false);
      setFoundAccounts([]);
      const uri = await pickVaultDirectory();
      if (!uri) {
        return;
      }
      // 开始同步时显示进度条
      setSyncPhase('syncing');
      setSyncFileName('');
      setSyncFileCount(0);

      // 监听 Kotlin 插件同步进度事件
      try {
        const { addPluginListener } = await import('@tauri-apps/api/core');
        if (unlistenSync.current) unlistenSync.current.unregister();
        unlistenSync.current = await addPluginListener<{
          phase: string;
          fileName?: string;
          fileCount?: number;
        }>('attachment-import-plugin', 'sync-progress', (payload) => {
          if (payload.phase === 'syncing') {
            setSyncFileName(payload.fileName ?? '');
            setSyncFileCount(payload.fileCount ?? 0);
          }
        });
      } catch {
        // 桌面端无 Kotlin 插件，静默失败
      }

      const result = await initVaultDirectory(uri);
      if (result.success) {
        if (result.accountCount && result.accountCount > 0) {
          // 已有账户：显示登录/创建决策卡片
          setFoundAccounts(result.accounts ?? []);
          setFoundAccountCount(result.accountCount ?? 0);
          setSyncPhase('idle');
          setShowAccountDecision(true);
          return;
        }
        // 显示 "SAF 同步完成" 3 秒后自动切换到路径显示
        syncDoneTimer.current = setTimeout(() => {
          setSyncPhase('idle');
          setSelectedSafUri(uri);
        }, 3000);
      } else {
        setSyncPhase('idle');
        setVaultDirError(result.message || t('onboarding_vault_dir_set_failed'));
      }
    } catch (e) {
      setSyncPhase('idle');
      setVaultDirError(String(e));
    } finally {
      // 确保在任何路径下都清理 Kotlin 插件事件监听
      if (unlistenSync.current) {
        unlistenSync.current.unregister();
        unlistenSync.current = null;
      }
      resume();
      setVaultDirActing(false);
    }
  }, [t]);

  const handleLoginExisting = useCallback(async () => {
    // 刷新全局账户状态，让 AppRoutes 自动路由到 /login
    await useAuthStore.getState().checkHasAccount();
    onComplete();
    navigate('/login', { replace: true });
  }, [navigate, onComplete]);

  const handleCreateNewAccount = useCallback(async () => {
    // 直接导航到创建账户页，由 AppRoutes 的特殊 query 处理
    onComplete();
    navigate('/bootstrap?mode=create', { replace: true });
  }, [navigate, onComplete]);

  // Show only the vault directory step when we need to display it
  if (current.key === 'vault_directory') {
    return (
      <div
        style={{
          position: 'fixed',
          inset: 0,
          zIndex: 'var(--z-onboarding)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'var(--bg-overlay)',
          backdropFilter: 'blur(4px)',
        }}
      >
        <div
          style={{
            background: 'var(--bg-elevated)',
            borderRadius: 18,
            padding: '32px 36px',
            maxWidth: 440,
            width: '90%',
            boxShadow: 'var(--shadow-lg)',
            border: '1px solid var(--border-subtle)',
            textAlign: 'center',
          }}
        >
          <div
            style={{
              width: 64,
              height: 64,
              borderRadius: 16,
              background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              margin: '0 auto 20px',
            }}
          >
            <Folder size={ICON_SIZE['3xl']} color="white" />
          </div>

          <h2
            style={{
              fontSize: 'var(--text-page-title)',
              fontWeight: 700,
              margin: '0 0 10px',
              color: 'var(--text-primary)',
            }}
          >
            {t('onboarding_vault_dir_title')}
          </h2>

          <p
            style={{
              fontSize: 'var(--text-body)',
              color: 'var(--text-secondary)',
              lineHeight: 1.6,
              margin: '0 0 24px',
            }}
          >
            {t('onboarding_vault_dir_desc')}
          </p>

          {showAccountDecision ? (
            /* 已有账户：让用户选择登录还是创建新账户 */
            <div
              style={{
                padding: 16,
                borderRadius: 12,
                border: '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)',
                background: 'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))',
                textAlign: 'left',
                marginBottom: 24,
              }}
            >
              <div
                style={{
                  fontSize: 'var(--text-body)',
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                  marginBottom: 8,
                }}
              >
                {t('onboarding_existing_accounts_title')}
              </div>
              <div
                style={{
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-secondary)',
                  marginBottom: 12,
                  lineHeight: 1.5,
                }}
              >
                {t('onboarding_existing_accounts_desc', {
                  count: foundAccountCount,
                })}
              </div>
              {foundAccounts.length > 0 && (
                <ul
                  style={{
                    margin: '0 0 12px 0',
                    paddingLeft: 18,
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.6,
                  }}
                >
                  {foundAccounts.slice(0, 3).map((acc) => (
                    <li key={acc.id}>{acc.name || acc.id}</li>
                  ))}
                  {foundAccounts.length > 3 && (
                    <li>{t('onboarding_existing_accounts_more', { count: foundAccounts.length - 3 })}</li>
                  )}
                </ul>
              )}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                <button
                  type="button"
                  onClick={handleLoginExisting}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 8,
                    padding: '12px 16px',
                    borderRadius: 10,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-primary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontWeight: 500,
                    fontSize: 'var(--text-body-sm)',
                    transition: 'all 0.15s ease',
                  }}
                  className="interactive-toolbar"
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-primary)';
                  }}
                >
                  <LogIn size={ICON_SIZE.md} />
                  {t('onboarding_action_login')}
                </button>
                <button
                  type="button"
                  onClick={handleCreateNewAccount}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 8,
                    padding: '12px 16px',
                    borderRadius: 10,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-primary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontWeight: 500,
                    fontSize: 'var(--text-body-sm)',
                    transition: 'all 0.15s ease',
                  }}
                  className="interactive-toolbar"
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-warm) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-warm)';
                    e.currentTarget.style.color = 'var(--accent-warm)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-primary)';
                  }}
                >
                  <UserPlus size={ICON_SIZE.md} />
                  {t('onboarding_action_create_new')}
                </button>
              </div>
            </div>
          ) : syncPhase === 'syncing' ? (
            /* SAF 同步中：进度条 + 提示 */
            <>
              <style>{`
                @keyframes sync-progress-bar {
                  0% { transform: translateX(-100%); }
                  50% { transform: translateX(200%); }
                  100% { transform: translateX(400%); }
                }
                @keyframes count-bounce {
                  0% { transform: scale(1.4); opacity: 0.7; }
                  40% { transform: scale(0.9); opacity: 1; }
                  70% { transform: scale(1.15); }
                  100% { transform: scale(1); }
                }
                @keyframes text-scroll {
                  0%, 15% { transform: translateX(0); }
                  85%, 100% { transform: translateX(calc(min(-100% + 280px, 0px))); }
                }
              `}</style>
              <div
                style={{
                  width: '100%',
                  height: 6,
                  borderRadius: 3,
                  background: 'var(--border-subtle)',
                  overflow: 'hidden',
                  marginBottom: 20,
                }}
              >
                <div
                  style={{
                    width: '30%',
                    height: '100%',
                    borderRadius: 3,
                    background: 'linear-gradient(90deg, var(--accent-primary), var(--accent-warm))',
                    animation: 'sync-progress-bar 1.5s ease-in-out infinite',
                  }}
                />
              </div>
              <div
                style={{
                  fontSize: 'var(--text-body)',
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                  marginBottom: 8,
                  overflow: 'hidden',
                  whiteSpace: 'nowrap',
                }}
              >
                {syncFileName ? (
                  <span
                    style={{
                      display: 'inline-block',
                      animation: 'text-scroll 4s ease-in-out infinite',
                      paddingRight: 8,
                    }}
                  >
                    {t('onboarding_vault_dir_syncing_file', {
                      fileName: syncFileName,
                      count: syncFileCount,
                    })}
                  </span>
                ) : (
                  t('onboarding_vault_dir_syncing')
                )}
              </div>
              <div
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  marginBottom: syncFileName ? 4 : 24,
                }}
              >
                {syncFileName
                  ? (
                    <span
                      key={syncFileCount}
                      style={{
                        display: 'inline-block',
                        animation: 'count-bounce 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)',
                      }}
                    >
                      {t('onboarding_vault_dir_sync_count', { count: syncFileCount })}
                    </span>
                  )
                  : t('onboarding_vault_dir_sync_hint')}
              </div>
            </>
          ) : syncPhase === 'done' ? (
            /* 同步完成：成功提示 */
            <div
              style={{
                padding: 16,
                borderRadius: 12,
                border: '1px solid color-mix(in srgb, var(--color-success, #22c55e) 35%, transparent)',
                background: 'color-mix(in srgb, var(--color-success, #22c55e) 8%, var(--bg-toolbar))',
                textAlign: 'center',
                marginBottom: 24,
              }}
            >
              <div style={{ fontSize: 32, marginBottom: 8 }}>✅</div>
              <div
                style={{
                  fontSize: 'var(--text-body)',
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                }}
              >
                {t('onboarding_vault_dir_sync_done')}
              </div>
            </div>
          ) : selectedSafUri ? (
            /* Selected SAF path summary */
            <div
              style={{
                padding: 16,
                borderRadius: 12,
                border: '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)',
                background: 'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))',
                textAlign: 'left',
                marginBottom: 24,
              }}
            >
              <div
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-secondary)',
                  marginBottom: 6,
                }}
              >
                {t('onboarding_vault_dir_selected_label')}
              </div>
              <div
                style={{
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-primary)',
                  wordBreak: 'break-all',
                  lineHeight: 1.5,
                }}
              >
                {selectedSafUri}
              </div>
              <button
                type="button"
                onClick={() => {
                  setSelectedSafUri(null);
                }}
                style={{
                  marginTop: 12,
                  fontSize: 'var(--text-caption)',
                  color: 'var(--accent-primary)',
                  background: 'transparent',
                  border: 'none',
                  padding: 0,
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  fontWeight: 500,
                }}
              >
                {t('onboarding_vault_dir_reselect')}
              </button>
            </div>
          ) : (
            <>
              {/* Choice: Local vs SAF */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 24 }}>
                <button
                  type="button"
                  onClick={async () => {
                    const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
                      (m) => m.useAutoLockPauseStore.getState(),
                    );
                    pause();
                    try {
                      setVaultDirActing(true);
                      const result = await initVaultDirectory(null);
                      if (result.success) {
                        setStep((s) => s + 1);
                      } else {
                        setVaultDirError(result.message || t('onboarding_vault_dir_set_failed'));
                      }
                    } catch (e) {
                      setVaultDirError(String(e));
                    } finally {
                      resume();
                      setVaultDirActing(false);
                    }
                  }}
                  style={{
                    padding: '14px 16px',
                    borderRadius: 12,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    cursor: 'pointer',
                    textAlign: 'left',
                    fontFamily: 'inherit',
                    transition: 'all 0.15s ease',
                  }}
                  className="interactive-toolbar"
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 8%, transparent)';
                    e.currentTarget.style.borderColor =
                      'color-mix(in srgb, var(--accent-primary) 40%, var(--border-subtle))';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }}
                >
                  <div style={{ fontWeight: 600, marginBottom: 4, color: 'var(--text-primary)' }}>
                    {t('onboarding_vault_dir_local_title')}
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-tertiary)',
                      lineHeight: 1.4,
                    }}
                  >
                    {t('onboarding_vault_dir_local_desc')}
                  </div>
                </button>

                <button
                  type="button"
                  onClick={handleVaultDirPick}
                  disabled={vaultDirActing}
                  style={{
                    padding: '14px 16px',
                    borderRadius: 12,
                    border: `1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)`,
                    background:
                      'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))',
                    cursor: vaultDirActing ? 'wait' : 'pointer',
                    textAlign: 'left',
                    fontFamily: 'inherit',
                    transition: 'all 0.15s ease',
                    opacity: vaultDirActing ? 0.6 : 1,
                  }}
                  onMouseEnter={(e) => {
                    if (!vaultDirActing) {
                      e.currentTarget.style.background =
                        'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!vaultDirActing) {
                      e.currentTarget.style.background =
                        'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))';
                    }
                  }}
                >
                  <div
                    style={{
                      fontWeight: 600,
                      marginBottom: 4,
                      color: 'var(--accent-primary)',
                    }}
                  >
                    {vaultDirActing
                      ? t('common:loading')
                      : t('onboarding_vault_dir_saf_title')}
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-secondary)',
                      lineHeight: 1.4,
                    }}
                  >
                    {t('onboarding_vault_dir_saf_desc')}
                  </div>
                </button>
              </div>

              {vaultDirError && (
                <div
                  style={{
                    padding: 8,
                    borderRadius: 8,
                    background: 'rgba(220, 38, 38, 0.08)',
                    color: '#dc2626',
                    fontSize: 'var(--text-body-sm)',
                    marginBottom: 16,
                  }}
                >
                  {vaultDirError}
                </div>
              )}
            </>
          )}

          {/* Step dots */}
          <div style={{ display: 'flex', justifyContent: 'center', gap: 6, marginBottom: 28 }}>
            {steps.map((_, i) => (
              <span
                key={i}
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  background: i === step ? 'var(--accent-primary)' : 'var(--border-subtle)',
                }}
              />
            ))}
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            {/* Hide skip button during vault directory step to prevent easy dismiss */}
            <div />
            <div style={{ display: 'flex', gap: 8 }}>
              {step > 0 && (
                <button
                  type="button"
                  onClick={() => {
                    setVaultDirError(null);
                    setStep((s) => s - 1);
                  }}
                  style={{
                    fontSize: 'var(--text-caption)',
                    padding: '6px 12px',
                    borderRadius: 6,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-primary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontWeight: 500,
                    transition: 'all 0.15s ease',
                  }}
                  className="interactive-toolbar"
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-primary)';
                  }}
                >
                  {t('onboarding_back')}
                </button>
              )}
              {selectedSafUri && (
                <button
                  type="button"
                  onClick={() => setStep((s) => s + 1)}
                  style={{
                    fontSize: 'var(--text-caption)',
                    padding: '6px 12px',
                    borderRadius: 6,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-primary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontWeight: 500,
                    transition: 'all 0.15s ease',
                  }}
                  className="interactive-toolbar"
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-primary)';
                  }}
                >
                  {t('onboarding_next')}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Regular step rendering
  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-onboarding)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
      }}
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 18,
          padding: '32px 36px',
          maxWidth: 440,
          width: '90%',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
          textAlign: 'center',
        }}
      >
        <div
          style={{
            width: 64,
            height: 64,
            borderRadius: 16,
            background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            margin: '0 auto 20px',
          }}
        >
          <Icon size={ICON_SIZE['3xl']} color="white" />
        </div>

        <h2
          style={{
            fontSize: 'var(--text-page-title)',
            fontWeight: 700,
            margin: '0 0 10px',
            color: 'var(--text-primary)',
          }}
        >
          {t(`onboarding_${current.key}_title`)}
        </h2>
        <p
          style={{
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            lineHeight: 1.6,
            margin: '0 0 28px',
            minHeight: 70,
          }}
        >
          {t(`onboarding_${current.key}_desc`)}
        </p>

        <div style={{ display: 'flex', justifyContent: 'center', gap: 6, marginBottom: 28 }}>
          {steps.map((_, i) => (
            <span
              key={i}
              style={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                background: i === step ? 'var(--accent-primary)' : 'var(--border-subtle)',
              }}
            />
          ))}
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div />
          <div style={{ display: 'flex', gap: 8 }}>
            {step > 0 && (
              <button
                type="button"
                onClick={() => setStep((s) => s - 1)}
                style={{
                  fontSize: 'var(--text-caption)',
                  padding: '6px 12px',
                  borderRadius: 6,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  color: 'var(--text-primary)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  fontWeight: 500,
                  transition: 'all 0.15s ease',
                }}
                className="interactive-toolbar"
                onMouseEnter={(e) => {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }}
              >
                {t('onboarding_back')}
              </button>
            )}
            <button
              type="button"
              onClick={() => {
                if (isLast) {
                  onComplete();
                } else {
                  setStep((s) => s + 1);
                }
              }}
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.color = 'var(--text-primary)';
              }}
            >
              {isLast ? t('onboarding_done') : t('onboarding_next')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
