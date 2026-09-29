import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import type { SensitivityLevel } from '@/types/template';
import { resolveFieldSensitivity } from '@/lib/fieldSensitivity';

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

interface SensitivityBadgesProps {
  properties: Array<{ id: string; sensitivityLevel?: string }>;
}

export function SensitivityBadges({ properties }: SensitivityBadgesProps) {
  const present = new Set(
    properties.map((p) => resolveFieldSensitivity({ fieldId: p.id, template: p })),
  );
  const ordered = SENSITIVITY_ORDER.filter((level) => present.has(level));
  if (ordered.length === 0) return null;
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
      {ordered.map((level) => (
        <SensitivityBadge key={level} level={level} />
      ))}
    </div>
  );
}
