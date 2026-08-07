/**
 * 敏感度掩码统一规则（P036）。
 *
 * 收敛三处各自实现的掩码判定（useRevealState / WorkspaceObjectCard /
 * HistoryViewer 规则与占位符此前不一致），落地 AGENTS.md §敏感数据分级
 * 与 P014 修订后的 4 级模型约定：
 * - `public` 永不掩码；
 * - `internal` / `sensitive` / `critical` 一律自动掩码（点击揭示）；
 * - 占位符统一为 8 圆点。
 */
import type { SensitivityLevel } from '@/types/template';

/** 敏感度掩码统一占位符（8 圆点）。 */
export const MASK_PLACEHOLDER = '••••••••';

/** 按敏感度判定是否应掩码（仅 public 放行）。 */
export function shouldMaskSensitivity(sensitivity: SensitivityLevel): boolean {
  return sensitivity !== 'public';
}

/** 按敏感度掩码值：命中掩码返回统一占位符，否则原样返回。 */
export function maskValue(value: string, sensitivity: SensitivityLevel): string {
  return shouldMaskSensitivity(sensitivity) ? MASK_PLACEHOLDER : value;
}
