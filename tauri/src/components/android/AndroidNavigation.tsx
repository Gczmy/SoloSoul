import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import {
  Home,
  Layers,
  Shapes,
  Settings,
  Plus,
  FilePlus2,
  FolderPlus,
  ScanLine,
  ArrowLeft,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AndroidSheet } from './AndroidSheet';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { IconCategoryPicker } from '@/components/layout/IconCategoryPicker';
import { useAddPageForm } from '@/hooks/useAddPageForm';
import { useToastError } from '@/hooks/useToastError';
import { useAuthStore } from '@/stores/authStore';
import { useAndroidCreateMenu } from '@/hooks/useAndroidCreateMenu';

export function androidDestination(path: string) {
  if (path === '/') return '/';
  if (path.startsWith('/workspace') || path.startsWith('/editor') || path === '/history')
    return '/workspace';
  if (path.startsWith('/settings')) return '/settings';
  return '/tools';
}

export function androidNewObjectUrl(path: string, search: string) {
  const page = path.match(/^\/workspace\/custom\/([^/]+)$/)?.[1];
  if (page) return `/editor?parentId=${encodeURIComponent(decodeURIComponent(page))}`;
  const section = new URLSearchParams(search).get('section');
  return path === '/workspace' && section
    ? `/editor?section=${encodeURIComponent(section)}`
    : '/editor';
}

function CreateSheet({
  onClose,
  trigger,
  newObjectUrl,
  startWithPage = false,
}: {
  onClose: () => void;
  trigger: HTMLElement | null;
  newObjectUrl: string;
  startWithPage?: boolean;
}) {
  const { t } = useTranslation(['navigation', 'common']);
  const navigate = useNavigate();
  const [pageForm, setPageForm] = useState(startWithPage);
  const pageFormRef = useRef(pageForm);
  pageFormRef.current = pageForm;
  const [submitting, setSubmitting] = useState(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError } = useToastError();
  const form = useAddPageForm({
    t,
    keepValuesOnSubmit: true,
    onError: (error, context) => {
      if (mounted.current) setSubmitting(false);
      onError(error, context);
    },
    onCreate: (page) => {
      // 异步创建完成时账户可能已锁定/切换，不能重新导航进受保护内容。
      if (
        mounted.current &&
        pageFormRef.current &&
        useAuthStore.getState().isAuthenticated &&
        useAuthStore.getState().currentAccount?.id === accountId
      ) {
        navigate(`/workspace/custom/${page.id}`);
        onClose();
      }
    },
  });
  const go = (path: string) => {
    navigate(path);
    onClose();
  };
  return (
    <AndroidSheet
      title={t(pageForm ? 'add_page' : 'common:material.new_title')}
      onClose={onClose}
      innerOpen={pageForm}
      onBack={() => setPageForm(false)}
      trigger={trigger}
    >
      {pageForm ? (
        <form
          className="android-page-form"
          onSubmit={(event) => {
            event.preventDefault();
            // 保存期间保留返回栈标记；导航完成再卸载，避免异步 go(-n) 吞掉新路由。
            if (!submitting && form.handleConfirm(true)) setSubmitting(true);
          }}
        >
          <button type="button" className="android-text-button" onClick={() => setPageForm(false)}>
            <ArrowLeft size={20} />
            {t('common:back')}
          </button>
          <Input
            disabled={submitting}
            autoFocus
            aria-label={t('add_page_placeholder')}
            placeholder={t('add_page_placeholder')}
            maxLength={20}
            value={form.name}
            onChange={(e) => form.setName(e.target.value)}
            error={
              form.nameError
                ? t(form.nameError === 'empty' ? 'page_name_required' : 'page_name_exists')
                : undefined
            }
          />
          <Input
            disabled={submitting}
            aria-label={t('add_page_description_placeholder')}
            placeholder={t('add_page_description_placeholder')}
            maxLength={30}
            value={form.description}
            onChange={(e) => form.setDescription(e.target.value)}
          />
          <p>{t('select_icon')}</p>
          <fieldset className="android-icon-picker" disabled={submitting}>
            <IconCategoryPicker
              selectedIconId={form.selectedIconId}
              onSelect={form.setSelectedIconId}
            />
          </fieldset>
          <Button type="submit" loading={submitting}>
            {t('common:create')}
          </Button>
        </form>
      ) : (
        <div className="android-create-options">
          <button type="button" data-initial-focus onClick={() => go(newObjectUrl)}>
            <FilePlus2 />
            <span>
              <strong>{t('common:material.new_object')}</strong>
              <small>{t('common:material.new_object_desc')}</small>
            </span>
          </button>
          <button type="button" onClick={() => setPageForm(true)}>
            <FolderPlus />
            <span>
              <strong>{t('add_page')}</strong>
              <small>{t('common:material.new_page_desc')}</small>
            </span>
          </button>
          <button type="button" onClick={() => go('/ocr')}>
            <ScanLine />
            <span>
              <strong>{t('common:material.scan')}</strong>
              <small>{t('common:material.scan_desc')}</small>
            </span>
          </button>
        </div>
      )}
    </AndroidSheet>
  );
}

export function AndroidNavigation() {
  const { t } = useTranslation(['navigation', 'common']);
  const { pathname, search, key } = useLocation();
  const [open, setOpen] = useState(false);
  const [startWithPage, setStartWithPage] = useState(false);
  const navigate = useNavigate();
  const trigger = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => setOpen(false), [key]);
  const destination = androidDestination(pathname);
  const nativeMenu = useAndroidCreateMenu((action) => {
    if (action === 'object') navigate(androidNewObjectUrl(pathname, search));
    else if (action === 'scan') navigate('/ocr');
    else {
      setStartWithPage(action === 'page');
      setOpen(true);
    }
  });
  const destinations = [
    { path: '/', Icon: Home, label: t('home') },
    { path: '/workspace', Icon: Layers, label: t('common:objects') },
    { path: '/tools', Icon: Shapes, label: t('common:material.tools') },
    { path: '/settings', Icon: Settings, label: t('settings') },
  ];
  const showFab = pathname === '/' || pathname.startsWith('/workspace') || pathname === '/tools';
  return (
    <>
      <nav
        className="android-navigation android-glass-surface"
        aria-label={t('common:material.navigation')}
      >
        {destinations.map(({ path, Icon, label }) => (
          <Link
            key={path}
            to={path}
            className="android-nav-item"
            aria-current={destination === path ? 'page' : undefined}
          >
            <span className="android-nav-icon">
              <Icon size={24} />
            </span>
            <span>{label}</span>
          </Link>
        ))}
      </nav>
      {showFab && (
        <button
          ref={trigger}
          type="button"
          className="android-fab"
          aria-haspopup="dialog"
          aria-expanded={open || nativeMenu.busy}
          aria-busy={nativeMenu.busy}
          disabled={nativeMenu.busy}
          onClick={() => void nativeMenu.open(trigger.current)}
        >
          <Plus size={24} />
          <span>{t('common:material.new_title')}</span>
        </button>
      )}
      {open && (
        <CreateSheet
          trigger={trigger.current}
          onClose={() => setOpen(false)}
          newObjectUrl={androidNewObjectUrl(pathname, search)}
          startWithPage={startWithPage}
        />
      )}
    </>
  );
}
