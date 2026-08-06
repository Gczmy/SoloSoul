import { useTranslation } from 'react-i18next';
import {
  LayoutList,
  Maximize2,
  Paperclip,
  Upload,
  LayoutTemplate,
  Shield,
  Pencil,
  Trash2,
  FileText,
  Settings,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

export interface GuidePage {
  icon: LucideIcon;
  title: string;
  steps: Array<{ icon: LucideIcon; title: string; description: string }>;
  helpLinks: Array<{ title: string; description: string; href: string }>;
}

/** 工作区页面的内置引导指南内容（静态数据，仅依赖 i18n）。 */
export function useWorkspaceGuidePages(): GuidePage[] {
  const { t } = useTranslation('common');
  return [
    {
      icon: LayoutList,
      title: t('common:guide_object_card_title', { defaultValue: '对象卡片指南' }),
      steps: [
        {
          icon: LayoutList,
          title: t('common:guide_card_step1_title', { defaultValue: '卡片结构' }),
          description:
            t('common:guide_card_step1_desc', { defaultValue: '每张对象卡片上方显示模板图标、对象名称和所属类别标签。若对象绑定了模板，还会显示模板名称。右侧是快捷操作按钮：历史记录（时钟图标）、附件列表（回形针图标）、编辑（铅笔图标）、删除（垃圾桶图标）。卡片主体以标签形式展示对象的字段属性。' }),
        },
        {
          icon: Shield,
          title: t('common:guide_card_step2_title', { defaultValue: '敏感度颜色' }),
          description:
            t('common:guide_card_step2_desc', { defaultValue: '字段的外边框颜色代表其敏感度等级：🟢 绿色 = 公开（public），🔵 蓝色 = 内部（internal），🟠 琥珀色 = 敏感（sensitive），🔴 红色 = 关键（critical）。非公开字段的值会自动模糊处理，保护隐私。' }),
        },
        {
          icon: Pencil,
          title: t('common:guide_card_step3_title', { defaultValue: '交互操作' }),
          description:
            t('common:guide_card_step3_desc', { defaultValue: '点击卡片任意区域打开详情面板查看所有字段。右上角的按钮分别用于：查看历史版本快照、管理附件、编辑对象、删除对象。将文件直接拖到卡片上可快速添加附件。' }),
        },
      ],
      helpLinks: [
        {
          title: t('common:guide_help_sensitivity', { defaultValue: '敏感度等级' }),
          description: t('common:guide_help_sensitivity_desc', { defaultValue: '了解不同敏感度等级的含义与安全策略' }),
          href: '/help?id=sensitivity',
        },
        {
          title: t('common:guide_help_objects', { defaultValue: '对象管理' }),
          description: t('common:guide_help_objects_desc', { defaultValue: '对象的创建、编辑、历史回溯与回收站管理' }),
          href: '/help?id=objects',
        },
        {
          title: t('common:guide_help_getting_started', { defaultValue: '快速开始' }),
          description: t('common:guide_help_getting_started_desc', { defaultValue: '了解 SoloSoul 基础操作与工作区布局' }),
          href: '/help?id=getting_started',
        },
      ],
    },
    {
      icon: LayoutTemplate,
      title: t('common:guide_template_title', { defaultValue: '对象模板指南' }),
      steps: [
        {
          icon: FileText,
          title: t('common:guide_tpl_step1_title', { defaultValue: '什么是模板' }),
          description:
            t('common:guide_tpl_step1_desc', { defaultValue: '模板定义了对象的数据结构，包含一组字段。每个字段有名称、类型（文本、数字、日期、选择等）和敏感度等级。使用模板创建对象时，系统会自动生成对应的字段，无需手动逐一添加。' }),
        },
        {
          icon: Settings,
          title: t('common:guide_tpl_step2_title', { defaultValue: '管理模板' }),
          description:
            t('common:guide_tpl_step2_desc', { defaultValue: '前往设置 > 模板管理器，可查看所有可用模板、创建新模板、编辑已有模板的字段结构。您可以为模板选择图标、设置所属页面、调整字段顺序和敏感度。已废弃的字段可以恢复或永久清理。' }),
        },
        {
          icon: Trash2,
          title: t('common:guide_tpl_step3_title', { defaultValue: '使用模板' }),
          description:
            t('common:guide_tpl_step3_desc', { defaultValue: '在新建对象时，可以选择一个模板来快速填充数据。对象创建后仍可更改或替换模板。如果模板被删除，已有对象的字段数据不会丢失，模板名称会显示为删除线。' }),
        },
      ],
      helpLinks: [
        {
          title: t('common:guide_help_templates', { defaultValue: '模板管理' }),
          description: t('common:guide_help_templates_desc', { defaultValue: '模板的创建、编辑、字段管理与废弃处理' }),
          href: '/help?id=templates',
        },
        {
          title: t('common:guide_help_create_object', { defaultValue: '创建对象' }),
          description: t('common:guide_help_create_object_desc', { defaultValue: '使用模板创建对象，快速录入结构化数据' }),
          href: '/help?id=objects',
        },
        {
          title: t('common:guide_help_objects', { defaultValue: '对象管理' }),
          description: t('common:guide_help_objects_desc', { defaultValue: '对象的创建、编辑、历史回溯与回收站管理' }),
          href: '/help?id=objects',
        },
      ],
    },
    {
      icon: Upload,
      title: t('common:drag_upload_guide_title', { defaultValue: '拖拽附件上传指南' }),
      steps: [
        {
          icon: LayoutList,
          title: t('common:drag_guide_step1_title', { defaultValue: '对象卡片' }),
          description:
            t('common:drag_guide_step1_desc', { defaultValue: '在工作区列表中，直接将文件拖拽到任意对象的卡片上，即可为该对象添加附件。拖入时卡片会高亮提示。' }),
        },
        {
          icon: Maximize2,
          title: t('common:drag_guide_step2_title', { defaultValue: '对象详情' }),
          description:
            t('common:drag_guide_step2_desc', { defaultValue: '点击对象卡片打开详情面板，将文件拖入面板内的任意区域，即可快速附加到当前对象。' }),
        },
        {
          icon: Paperclip,
          title: t('common:drag_guide_step3_title', { defaultValue: '附件管理' }),
          description:
            t('common:drag_guide_step3_desc', { defaultValue: '在附件管理器弹窗中，直接将文件拖入窗口，即可批量上传多个附件。支持同时拖入多个文件。' }),
        },
      ],
      helpLinks: [
        {
          title: t('common:guide_help_getting_started', { defaultValue: '快速开始' }),
          description: t('common:guide_help_getting_started_desc', { defaultValue: '了解 SoloSoul 基础操作与工作区布局' }),
          href: '/help?id=getting_started',
        },
        {
          title: t('common:guide_help_attachments', { defaultValue: '附件管理' }),
          description: t('common:guide_help_attachments_desc', { defaultValue: '附件的上传、下载、重命名与回收站管理' }),
          href: '/help?id=attachments',
        },
        {
          title: t('common:guide_help_sensitivity', { defaultValue: '敏感度等级' }),
          description: t('common:guide_help_sensitivity_desc', { defaultValue: '了解不同敏感度等级的含义与安全策略' }),
          href: '/help?id=sensitivity',
        },
      ],
    },
  ];
}
