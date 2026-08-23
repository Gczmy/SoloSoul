/**
 * P009：导出/导入页引导步骤配置（自 useExportImportPage 拆出）。
 */
import { Info, FolderOpen, Lock, GitCompare } from 'lucide-react';
import type { TFunction } from 'i18next';

export function buildExportImportGuidePages(t: TFunction) {
  return [
    {
      icon: Info,
      title: t('common:guide_export_import_title', { defaultValue: 'Export & Import Guide' }),
      steps: [
        {
          icon: FolderOpen,
          title: t('common:guide_export_import_step1_title', { defaultValue: 'Select Scope' }),
          description:
            t('common:guide_export_import_step1_desc', { defaultValue: 'Choose the pages, objects, and tags you want to export. You can also include attachments, preferences, and behavioral data.' }),
        },
        {
          icon: Lock,
          title: t('common:guide_export_import_step2_title', { defaultValue: 'Set Password' }),
          description:
            t('common:guide_export_import_step2_desc', { defaultValue: 'Exports are encrypted with a password you provide. Keep the password safe — you will need it to import the package later.' }),
        },
        {
          icon: GitCompare,
          title: t('common:guide_export_import_step3_title', { defaultValue: 'Import & Strategy' }),
          description:
            t('common:guide_export_import_step3_desc', { defaultValue: 'When importing, preview the package and choose how to handle duplicate objects: skip existing, overwrite, or decide per object.' }),
        },
      ],
      helpLinks: [
        {
          title: t('common:guide_help_export_import', { defaultValue: 'Export & Import' }),
          description:
            t('common:guide_help_export_import_desc', { defaultValue: 'Encrypted export and import of your vault data' }),
          href: '/help?id=export_import',
        },
      ],
    },
  ];
}
