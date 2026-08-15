import { Info, Lock, Eye, History, Upload, Maximize2, Paperclip } from 'lucide-react';
import type { useTranslation } from 'react-i18next';
import type { GuidePage } from '@/components/guide/PageGuide';
import {
  flattenPropertyEntries,
  type FlattenedPropertyEntry,
} from '@/lib/propertyFlatten';

/** 对象详情可展示字段条目（dynamic_group 子字段以独立条目 + label 返回）。 */
export type FlattenedObjectDetailField = {
  key: string;
  label?: string;
  value: string;
  fieldId?: string;
};

/**
 * P024: 收敛至共享核心 flattenPropertyEntries（展平模式 + 过滤 `__` 字段）。
 * - dynamic_group 类型：每个子字段作为独立条目返回，使用子字段名称作为 label；
 * - 支持通过 fieldDefs 参数显式传入字段定义（否则回退到 properties.__fields）。
 */
export function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
  fieldDefs?: Record<string, { type?: string }>,
): FlattenedObjectDetailField[] {
  // 展平模式仅产生 field 条目；filter 收窄 union 后取 value/fieldId。
  return flattenPropertyEntries(props, fieldOrder, fieldDefs)
    .filter((e): e is Extract<FlattenedPropertyEntry, { kind: 'field' }> => e.kind === 'field')
    .map((e) => ({ key: e.key, label: e.label, value: e.value, fieldId: e.fieldId }));
}

/** P041: 指南内容数据（移动端 / 桌面端两套），纯函数便于单测。 */
export function buildDetailGuidePages(
  t: ReturnType<typeof useTranslation>['t'],
  isMobilePlatform: boolean,
): GuidePage[] {
  return isMobilePlatform
    ? [
        {
          icon: Info,
          title: t('common:guide_detail_mobile_title', { defaultValue: '对象详情卡片' }),
          steps: [
            {
              icon: Eye,
              title: t('common:guide_detail_mobile_step1_title', { defaultValue: '字段与敏感等级' }),
              description:
                t('common:guide_detail_mobile_step1_desc', { defaultValue: '详情卡片会列出对象的所有字段，显示字段名称、类型图标和敏感度标签。敏感/关键字段的值默认会被遮罩，以保护隐私。' }),
            },
            {
              icon: Lock,
              title: t('common:guide_detail_mobile_step2_title', { defaultValue: '显示与解锁' }),
              description:
                t('common:guide_detail_mobile_step2_desc', { defaultValue: '点击敏感字段旁的「显示」图标可查看内容；关键字段旁会显示「解锁」图标，需通过主密码、PIN 或生物识别验证后才能临时查看。' }),
            },
            {
              icon: History,
              title: t('common:guide_detail_mobile_step3_title', { defaultValue: '操作按钮' }),
              description:
                t('common:guide_detail_mobile_step3_desc', { defaultValue: '卡片底部提供四个常用操作：历史记录（时钟图标）查看版本快照、附件（回形针图标）管理文件、编辑（铅笔图标）进入编辑器、删除（垃圾桶图标）将对象移入回收站。' }),
            },
          ],
          helpLinks: [
            {
              title: t('common:guide_help_sensitivity', { defaultValue: '敏感度等级' }),
              description:
                t('common:guide_help_sensitivity_desc', { defaultValue: '了解不同敏感度等级的含义与安全策略' }),
              href: '/help?id=sensitivity',
            },
            {
              title: t('common:guide_help_attachments', { defaultValue: '附件管理' }),
              description:
                t('common:guide_help_attachments_desc', { defaultValue: '附件的上传、下载、重命名与回收站管理' }),
              href: '/help?id=attachments',
            },
          ],
        },
      ]
    : [
        {
          icon: Upload,
          title: t('common:drag_upload_guide_title', { defaultValue: '拖拽附件上传指南' }),
          steps: [
            {
              icon: Maximize2,
              title: t('common:guide_detail_step1_title', { defaultValue: '拖拽到此面板' }),
              description:
                t('common:guide_detail_step1_desc', { defaultValue: '直接将文件从文件管理器拖入当前详情面板，即可为此对象添加附件。拖入时面板会高亮提示。' }),
            },
            {
              icon: Paperclip,
              title: t('common:guide_detail_step2_title', { defaultValue: '附件管理器' }),
              description:
                t('common:guide_detail_step2_desc', { defaultValue: '点击「附件」按钮打开附件管理器，也可将文件直接拖入管理器窗口进行批量上传。' }),
            },
          ],
          helpLinks: [
            {
              title: t('common:guide_help_attachments', { defaultValue: '附件管理' }),
              description:
                t('common:guide_help_attachments_desc', { defaultValue: '附件的上传、下载、重命名与回收站管理' }),
              href: '/help?id=attachments',
            },
          ],
        },
      ];
}
