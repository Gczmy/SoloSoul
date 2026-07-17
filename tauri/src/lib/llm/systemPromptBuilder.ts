// =============================================================================
// 系统提示词构建器（模式 A — 前端构建）
// =============================================================================
// 根据文档 §6 规范，在前端利用已有 Zustand Store 数据构建系统提示词。
// 包含 7 个 Section：AI 身份 / 软件信息 / 用户公开对象数据 / 偏好 / 插件 / 统计 / 行为规范
// =============================================================================

import i18n from '@/lib/i18n';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';

const APP_NAME = 'SoloSoul（独灵）';
const AI_NAME = 'Solon';

// 从环境或构建时注入的版本信息（Vite 方式）
const APP_VERSION = (import.meta.env.VITE_APP_VERSION as string | undefined) ?? '2.0.0';

function getPlatform(): string {
  const nav = navigator as Navigator & { userAgentData?: { platform: string } };
  if (nav.userAgentData?.platform) return nav.userAgentData.platform;
  if (/Mac/i.test(navigator.platform)) return 'macOS';
  if (/Win/i.test(navigator.platform)) return 'Windows';
  if (/Linux/i.test(navigator.platform)) return 'Linux';
  return navigator.platform || 'Unknown';
}

function getLanguage(): string {
  return i18n.language || 'zh-CN';
}

// ── Section 构建器 ──────────────────────────────────────────────────────────

function buildSection1Identity(): string {
  return `你是 ${APP_NAME} 的 AI 助手 ${AI_NAME}，由 SoloSoul 团队开发。
你是用户的个人智能助手，了解用户的个人信息（仅限用户主动分享的部分）。
你的回答应当简洁、准确、有帮助。`;
}

function buildSection2SoftwareInfo(): string {
  return `当前 SoloSoul 版本：${APP_VERSION}
平台：${getPlatform()}
界面语言：${getLanguage()}`;
}

function buildSection3PublicObjectData(): string {
  const objects = useObjectStore.getState().objects;
  const publicObjects = objects
    .filter((o) => o.sensitivityLevel === 'public' && !o.isDeleted)
    .slice(0, 3); // 最多 3 个对象

  if (publicObjects.length === 0) {
    return '（用户尚未公开任何对象数据）';
  }

  const lines: string[] = [];
  for (const obj of publicObjects) {
    const props = obj.properties || {};
    const propEntries = Object.entries(props)
      .slice(0, 8) // 每对象最多 8 个属性
      .map(([k, v]) => {
        const str = String(v ?? '');
        return `${k}: ${str.length > 100 ? str.slice(0, 100) + '…' : str}`;
      });
    lines.push(`${obj.collectionType}（${obj.name}）：${propEntries.join('、') || '（无属性）'}`);
  }
  return lines.join('\n');
}

function buildSection4Preferences(): string {
  const s = useSettingsStore.getState().settings;
  const items: string[] = [];
  if (s.theme) items.push(`主题：${s.theme}`);
  if (s.language) items.push(`语言：${s.language}`);
  if (s.accentColor) items.push(`主题色：${s.accentColor}`);
  if (s.autoLockTimeoutMinutes) items.push(`自动锁定：${s.autoLockTimeoutMinutes} 分钟`);
  return items.length > 0 ? items.join('\n') : '（无特殊偏好设置）';
}

function buildSection5Plugins(): string {
  // 插件系统尚未完全实现，返回占位
  return '（暂无已安装插件）';
}

function buildSection6UsageStats(): string {
  // 统计功能尚未实现，返回占位
  return '（使用统计功能即将上线）';
}

function buildSection7BehaviorGuidelines(): string {
  return `1. 使用与用户提问相同的语言回答，语气自然、亲切、生动，像一位熟悉软件的朋友在帮忙
2. 区分"插件"（功能扩展）和"对象"（用户数据）
3. 敏感/受限/关键数据需要重新验证密码，无法直接访问
4. 优先依据下方提供的帮助文档回答，允许用自然语言转述文档内容，禁止编造文档中没有的信息（如快捷键、菜单路径、不存在的按钮）。只有文档中完全没有相关内容时，才回答"不清楚"
5. 不泄露用户数据给插件或外部服务
6. 禁止使用"根据你使用的 SoloSoul 软件环境"、"在当前版本中"这类生硬开场白，直接给出操作步骤即可`;
}

// ── 主构建函数 ─────────────────────────────────────────────────────────────

export interface SystemPromptOptions {
  includeSoftwareInfo?: boolean;
  includePublicProfile?: boolean;
  includePreferences?: boolean;
  includePlugins?: boolean;
  includeUsageStats?: boolean;
  includeBehaviorGuidelines?: boolean;
}

export function buildSystemPrompt(options: SystemPromptOptions = {}): string {
  const {
    includeSoftwareInfo = true,
    includePublicProfile = true,
    includePreferences = true,
    includePlugins = true,
    includeUsageStats = true,
    includeBehaviorGuidelines = true,
  } = options;

  const sections: string[] = [];
  sections.push(`【Section 1: AI 身份定义】\n${buildSection1Identity()}`);

  if (includeSoftwareInfo) {
    sections.push(`【Section 2: 软件信息】\n${buildSection2SoftwareInfo()}`);
  }
  if (includePublicProfile) {
    sections.push(`【Section 3: 用户公开对象数据】\n${buildSection3PublicObjectData()}`);
  }
  if (includePreferences) {
    sections.push(`【Section 4: 偏好设置】\n${buildSection4Preferences()}`);
  }
  if (includePlugins) {
    sections.push(`【Section 5: 已安装插件】\n${buildSection5Plugins()}`);
  }
  if (includeUsageStats) {
    sections.push(`【Section 6: 使用统计】\n${buildSection6UsageStats()}`);
  }
  if (includeBehaviorGuidelines) {
    sections.push(`【Section 7: 行为规范】\n${buildSection7BehaviorGuidelines()}`);
  }

  let prompt = sections.join('\n\n');

  // 长度限制：系统提示词上限 1500 字符
  const MAX_SYSTEM_PROMPT_CHARS = 1500;
  if (prompt.length > MAX_SYSTEM_PROMPT_CHARS) {
    // 优先截断 Section 3（用户公开对象数据），保留核心部分
    const truncationNotice = '\n\n（上下文过长，部分内容已省略）';
    const maxLen = MAX_SYSTEM_PROMPT_CHARS - truncationNotice.length;
    prompt = prompt.slice(0, maxLen);
    // 在行边界截断
    const lastNewline = prompt.lastIndexOf('\n');
    if (lastNewline > maxLen * 0.7) {
      prompt = prompt.slice(0, lastNewline);
    }
    prompt += truncationNotice;
  }

  return prompt;
}

// ── 快捷函数：组装完整 messages ────────────────────────────────────────────

export interface ChatMessage {
  role: string;
  content: string;
}

export function buildMessagesWithSystemPrompt(
  userPrompt: string,
  history: ChatMessage[],
  systemPrompt: string,
): ChatMessage[] {
  const messages: ChatMessage[] = [];
  messages.push({ role: 'system', content: systemPrompt });
  messages.push(...history);
  messages.push({ role: 'user', content: userPrompt });
  return messages;
}

/**
 * 将 Help Doc 合并到单条 system message 中，避免多 system message 导致模型忽略。
 * Help Doc 放在 system prompt 之后，用明确的分隔线隔开。
 */
export function buildMessagesWithSystemPromptAndGuide(
  userPrompt: string,
  history: ChatMessage[],
  systemPrompt: string,
  guideContent: string | null,
): ChatMessage[] {
  let combinedSystem = systemPrompt;
  if (guideContent) {
    combinedSystem = `${systemPrompt}\n\n${guideContent}`;
  }
  // 总长度限制：单条 system message 不超过 3000 字符
  const MAX_TOTAL = 3000;
  if (combinedSystem.length > MAX_TOTAL) {
    const notice = '\n\n（以下内容因长度限制被截断）';
    combinedSystem = combinedSystem.slice(0, MAX_TOTAL - notice.length) + notice;
  }
  return buildMessagesWithSystemPrompt(userPrompt, history, combinedSystem);
}
