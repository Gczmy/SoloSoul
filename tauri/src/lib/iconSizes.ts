/**
 * 语义化图标尺寸常量。
 *
 * 与 CSS token 对应：
 *   --icon-2xs: 10px; --icon-xs: 12px; --icon-sm: 14px;
 *   --icon-md: 16px; --icon-lg: 18px; --icon-xl: 20px;
 *   --icon-2xl: 24px; --icon-3xl: 32px; --icon-4xl: 40px;
 *   --icon-5xl: 48px; --icon-6xl: 72px;
 *
 * 推荐用法：
 *   <SomeIcon size={ICON_SIZE.sm} />
 *   <BadgeIconButton iconSize={ICON_SIZE.xs} />
 */
export const ICON_SIZE = {
  /** 10px — 极小内联图标、徽标按钮内图标 */
  '2xs': 10,
  /** 12px — 小图标：列表行、折叠箭头、小状态 */
  xs: 12,
  /** 14px — 默认内联 / 按钮 / 导航 / 列表 */
  sm: 14,
  /** 16px — 卡片标题 / 表单标签 / 常规状态 */
  md: 16,
  /** 18px — 区块标题 / 大按钮 / 预览工具栏 */
  lg: 18,
  /** 20px — 页面标题 / 大型操作 */
  xl: 20,
  /** 24px — 对话框 / 活动面板 / 上传区域 */
  '2xl': 24,
  /** 32px — 空状态 / 大徽标 / 侧边栏 Logo */
  '3xl': 32,
  /** 40px — 超大空状态 / 启动页图标 */
  '4xl': 40,
  /** 48px — 巨型空状态 / 品牌图标 */
  '5xl': 48,
  /** 72px — 启动页 Logo / 品牌标识 */
  '6xl': 72,
} as const;
