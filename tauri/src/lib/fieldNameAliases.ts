/**
 * 多语言字段名规范解析（跨语言同名字段匹配）。
 *
 * 系统模板（system_templates_*.json）的字段 id 即规范字段名，显示名经
 * `editor:fields.<id>` 本地化——同一字段在不同语言下名字不同（如 dateOfBirth
 * → 「出生日期」/「Date of Birth」）。字段推荐（object_field_suggestions）
 * 需要把不同语言的同名字段视为同一字段：这里以 zh-CN 与 en-US 的 editor.json
 * `fields` 映射为唯一真理来源（与 lib/i18n.ts 同源），构建「本地化名 → 规范 id」
 * 反向映射；字段 key 本身命中已知规范 id 时也按 key 归一（覆盖模板重命名场景）。
 */
import zhEditor from '@/locales/zh-CN/editor.json';
import enEditor from '@/locales/en-US/editor.json';

interface EditorDict {
  fields: Record<string, string>;
}

/** 规范字段 id 集合（editor:fields 已知的系统字段）。 */
const CANONICAL_IDS: Set<string> = new Set();
/** 本地化字段名 → 规范字段 id（同一名字映射到多个 id 时先到先得，确定性）。 */
const LOCALIZED_NAME_TO_CANONICAL: Map<string, string> = new Map();

for (const dict of [zhEditor, enEditor] as EditorDict[]) {
  for (const [id, name] of Object.entries(dict.fields)) {
    if (id === '__dynamic_group__') continue;
    CANONICAL_IDS.add(id);
    const trimmed = name.trim();
    if (trimmed && !LOCALIZED_NAME_TO_CANONICAL.has(trimmed)) {
      LOCALIZED_NAME_TO_CANONICAL.set(trimmed, id);
    }
  }
}

/**
 * 把「字段 key + 显示名」解析为语言无关的规范字段名，解析顺序：
 * 1. 字段 key 命中已知规范 id（系统模板跨语言同 key，即使名字被重命名）→ 用 key；
 * 2. 显示名命中任意语言的已知本地化名 → 映射回规范 id（同名字不同 key 的跨语言匹配）；
 * 3. 兜底：显示名原样返回（去首尾空白）。
 *
 * 示例：key=dateOfBirth + name=「出生日期」与 key=dateOfBirth + name=「Date of
 * Birth」均解析为 dateOfBirth；key=id_number + name=「ID Number」解析为 idNumber。
 */
export function resolveCanonicalFieldName(fieldKey: string, fieldName: string): string {
  const key = fieldKey.trim();
  const name = fieldName.trim();
  if (key && CANONICAL_IDS.has(key)) return key;
  return name ? (LOCALIZED_NAME_TO_CANONICAL.get(name) ?? name) : name;
}
