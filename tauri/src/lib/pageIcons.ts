import {
  Home,
  IdCard,
  Plane,
  Banknote,
  Briefcase,
  Search,
  Settings,
  Lock,
  Unlock,
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
  Trash2,
  LayoutTemplate,
  ArrowLeftRight,
  Shield,
  Key,
  CreditCard,
  Wallet,
  Coins,
  Receipt,
  MapPin,
  Compass,
  Hotel,
  Building2,
  User,
  Users,
  Smartphone,
  Wifi,
  Sun,
  Moon,
  Cloud,
  Scan,
  // Batch 1 — 通用扩展
  Award,
  BadgeCheck,
  Brain,
  Calculator,
  Camera,
  Car,
  ClipboardList,
  Contact,
  DollarSign,
  Cross,
  Fingerprint,
  Gem,
  HeartPulse,
  Landmark,
  Laptop,
  Lightbulb,
  Luggage,
  Mail,
  Navigation,
  PawPrint,
  PenLine,
  Percent,
  PhoneCall,
  PieChart,
  Pill,
  Printer,
  ScrollText,
  Send,
  Signature,
  Stethoscope,
  TreePine,
  TrendingUp,
  Tv,
  // Batch 2 — 证件/教育/实用
  BookMarked,
  Crown,
  Eye,
  GraduationCap,
  Link,
  Notebook,
  Rocket,
  Sparkles,
  Target,
  Ticket,
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
  trash: Trash2,
  templates: LayoutTemplate,
  import_export: ArrowLeftRight,
  ocr: Scan,
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
  // 安全
  shield: Shield,
  key: Key,
  lock: Lock,
  unlock: Unlock,
  fingerprint: Fingerprint,
  badge_check: BadgeCheck,
  // 身份/证件
  scroll: ScrollText,
  signature: Signature,
  contact: Contact,
  id_card: IdCard,
  bookmarked: BookMarked,
  ticket: Ticket,
  // 财务
  credit_card: CreditCard,
  wallet: Wallet,
  coins: Coins,
  receipt: Receipt,
  dollar: DollarSign,
  landmark: Landmark,
  pie_chart: PieChart,
  trending_up: TrendingUp,
  calculator: Calculator,
  percent: Percent,
  // 旅行/交通
  plane: Plane,
  map_pin: MapPin,
  compass: Compass,
  hotel: Hotel,
  car: Car,
  luggage: Luggage,
  navigation: Navigation,
  // 工作/技术
  building: Building2,
  user: User,
  users: Users,
  briefcase: Briefcase,
  laptop: Laptop,
  printer: Printer,
  award: Award,
  clipboard: ClipboardList,
  camera: Camera,
  pen: PenLine,
  // 沟通
  mail: Mail,
  phone: PhoneCall,
  send: Send,
  // 医疗/健康
  heart_pulse: HeartPulse,
  brain: Brain,
  stethoscope: Stethoscope,
  first_aid: Cross,
  pill: Pill,
  // 教育
  graduation: GraduationCap,
  notebook: Notebook,
  // 生活
  smartphone: Smartphone,
  wifi: Wifi,
  sun: Sun,
  moon: Moon,
  cloud: Cloud,
  home: Home,
  tv: Tv,
  lightbulb: Lightbulb,
  eye: Eye,
  link: Link,
  // 自然/生物
  gem: Gem,
  tree: TreePine,
  paw: PawPrint,
  // 特色
  crown: Crown,
  rocket: Rocket,
  sparkles: Sparkles,
  target: Target,
} as const satisfies Record<string, LucideIcon>;

export type CustomIconId = keyof typeof CUSTOM_ICON_MAP;

export const DEFAULT_CUSTOM_ICON: CustomIconId = 'document';

/**
 * 根据 iconId 解析自定义页面图标。
 * 若 iconId 不在映射表中则回退到默认 document 图标。
 */
/**
 * 图标分类映射表 — 将每个图标 ID 映射到所属分类。
 * 用于图标选择器的分类筛选。
 */
export const ICON_CATEGORIES: Record<CustomIconId, string> = {
  // 通用
  document: 'general',
  folder: 'general',
  star: 'general',
  heart: 'general',
  bookmark: 'general',
  globe: 'general',
  code: 'general',
  image: 'general',
  music: 'general',
  cart: 'general',
  coffee: 'general',
  gift: 'general',
  calendar: 'general',
  map: 'general',
  flag: 'general',
  // 安全
  shield: 'security',
  key: 'security',
  lock: 'security',
  unlock: 'security',
  fingerprint: 'security',
  badge_check: 'security',
  // 身份/证件
  scroll: 'identity',
  signature: 'identity',
  contact: 'identity',
  id_card: 'identity',
  bookmarked: 'identity',
  ticket: 'identity',
  // 财务
  credit_card: 'finance',
  wallet: 'finance',
  coins: 'finance',
  receipt: 'finance',
  dollar: 'finance',
  landmark: 'finance',
  pie_chart: 'finance',
  trending_up: 'finance',
  calculator: 'finance',
  percent: 'finance',
  // 旅行/交通
  plane: 'travel',
  map_pin: 'travel',
  compass: 'travel',
  hotel: 'travel',
  car: 'travel',
  luggage: 'travel',
  navigation: 'travel',
  // 工作/技术
  building: 'work',
  user: 'work',
  users: 'work',
  briefcase: 'work',
  laptop: 'work',
  printer: 'work',
  award: 'work',
  clipboard: 'work',
  camera: 'work',
  pen: 'work',
  // 沟通
  mail: 'communication',
  phone: 'communication',
  send: 'communication',
  // 医疗/健康
  heart_pulse: 'health',
  brain: 'health',
  stethoscope: 'health',
  first_aid: 'health',
  pill: 'health',
  // 教育
  graduation: 'education',
  notebook: 'education',
  // 生活
  smartphone: 'life',
  wifi: 'life',
  sun: 'life',
  moon: 'life',
  cloud: 'life',
  home: 'life',
  tv: 'life',
  lightbulb: 'life',
  eye: 'life',
  link: 'life',
  // 自然/生物
  gem: 'nature',
  tree: 'nature',
  paw: 'nature',
  // 特色
  crown: 'special',
  rocket: 'special',
  sparkles: 'special',
  target: 'special',
};

/**
 * 分类显示标签映射表。
 */
export const CATEGORY_LABELS: Record<string, string> = {
  all: '全部',
  general: '通用',
  security: '安全',
  identity: '身份/证件',
  finance: '财务',
  travel: '旅行/交通',
  work: '工作/技术',
  communication: '沟通',
  health: '医疗/健康',
  education: '教育',
  life: '生活',
  nature: '自然/生物',
  special: '特色',
};

export function resolveCustomIcon(iconId: string): LucideIcon {
  if (iconId in CUSTOM_ICON_MAP) {
    return CUSTOM_ICON_MAP[iconId as CustomIconId];
  }
  return CUSTOM_ICON_MAP[DEFAULT_CUSTOM_ICON];
}
