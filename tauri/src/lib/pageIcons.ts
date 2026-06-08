import {
  Home,
  IdCard,
  Plane,
  Banknote,
  Briefcase,
  Search,
  Settings,
  Lock,
  Puzzle,
  MessageSquare,
  FileText,
  Folder,
  Star,
  Heart,
  Bookmark,
  Globe,
  Code2,
  Image,
  Music,
  ShoppingCart,
  Coffee,
  Gift,
  Calendar,
  Map,
  Flag,
  BookOpen,
  type LucideIcon,
} from 'lucide-react';

// =============================================================================
// §7.4 — 页面图标唯一映射规范（Single Source of Truth）
// =============================================================================

/**
 * 系统固定图标映射表（不可由用户更改）。
 * 同一页面在所有位置（侧边栏、主页、设置页等）必须使用此处定义的同一图标。
 */
export const PAGE_ICON_MAP = {
  home: Home,
  profile: IdCard,
  identity: IdCard,
  travel: Plane,
  financial: Banknote,
  professional: Briefcase,
  search: Search,
  settings: Settings,
  lock: Lock,
  plugins: Puzzle,
  ai_chat: MessageSquare,
  help: BookOpen,
  custom: FileText,
} as const satisfies Record<string, LucideIcon>;

export type PageIconKey = keyof typeof PAGE_ICON_MAP;

/**
 * 用户可自定义图标映射表（用于自定义页面和分区的图标选择器）。
 * 仅包含允许用户选择的图标，不含系统固定功能图标。
 */
export const CUSTOM_ICON_MAP = {
  document: FileText,
  folder: Folder,
  star: Star,
  heart: Heart,
  bookmark: Bookmark,
  globe: Globe,
  code: Code2,
  image: Image,
  music: Music,
  cart: ShoppingCart,
  coffee: Coffee,
  gift: Gift,
  calendar: Calendar,
  map: Map,
  flag: Flag,
} as const satisfies Record<string, LucideIcon>;

export type CustomIconId = keyof typeof CUSTOM_ICON_MAP;

export const DEFAULT_CUSTOM_ICON: CustomIconId = 'document';

/**
 * 根据 iconId 解析自定义页面图标。
 * 若 iconId 不在映射表中则回退到默认 document 图标。
 */
export function resolveCustomIcon(iconId: string): LucideIcon {
  if (iconId in CUSTOM_ICON_MAP) {
    return CUSTOM_ICON_MAP[iconId as CustomIconId];
  }
  return CUSTOM_ICON_MAP[DEFAULT_CUSTOM_ICON];
}
