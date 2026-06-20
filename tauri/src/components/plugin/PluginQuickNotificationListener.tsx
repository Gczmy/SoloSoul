import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { usePluginStore } from '@/stores/pluginStore';
import { usePluginQuickStore } from '@/stores/pluginQuickStore';
import { useUiStore } from '@/stores/uiStore';

/**
 * PluginQuickNotificationListener — watches for background plugin run completion
 * and shows a toast notification when the quick panel is closed.
 */
export function PluginQuickNotificationListener() {
  const { t } = useTranslation('plugin');
  const showToast = useUiStore((s) => s.showToast);
  const runningPlugins = usePluginStore((s) => s.runningPlugins);
  const isOpen = usePluginQuickStore((s) => s.isOpen);

  const initializedRef = useRef(false);
  const prevCompletedRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    // On first mount, initialize prevCompletedRef with already-completed plugins
    // to prevent toast flood from previously persisted runs.
    if (!initializedRef.current) {
      for (const [id, p] of Object.entries(runningPlugins)) {
        if (p.completed) prevCompletedRef.current.add(id);
      }
      initializedRef.current = true;
      return;
    }

    const currentCompleted = new Set<string>();

    for (const [id, plugin] of Object.entries(runningPlugins)) {
      if (plugin.completed) {
        currentCompleted.add(id);

        // If this plugin just completed while panel is closed
        if (!prevCompletedRef.current.has(id) && !isOpen) {
          if (plugin.error) {
            showToast({
              type: 'error',
              message: t('plugin:run_failed', {
                pluginName: plugin.pluginName,
                defaultValue: `「${plugin.pluginName}」plugin run failed`,
              }),
              duration: 5000,
            });
          } else {
            showToast({
              type: 'success',
              message: t('plugin:run_complete', {
                pluginName: plugin.pluginName,
                defaultValue: `「${plugin.pluginName}」plugin run completed`,
              }),
              duration: 3000,
            });
          }
        }
      }
    }

    prevCompletedRef.current = currentCompleted;
  }, [runningPlugins, isOpen, showToast, t]);

  return null;
}
