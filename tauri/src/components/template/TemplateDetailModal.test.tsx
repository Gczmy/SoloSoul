import { render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { usePluginStore } from '@/stores/pluginStore';
import { TemplateDetailModal } from './TemplateDetailModal';

it('非法敏感度在模板汇总和详情字段中均显示为 internal', () => {
  usePluginStore.setState({ loadInstalled: vi.fn().mockResolvedValue(undefined) });
  render(
    <TemplateDetailModal
      detailTemplate={{
        id: 'tpl-1',
        name: '旧模板',
        category: 'identity',
        properties: [
          { id: 'bad', name: '旧字段', type: 'text', sensitivityLevel: 'unknown' },
          { id: 'critical', name: '关键字段', type: 'text', sensitivityLevel: 'critical' },
        ],
      }}
      templates={[]}
      pageLabel={() => ({ name: '身份', deleted: false })}
      onClose={vi.fn()}
      onEdit={vi.fn()}
    />,
  );

  expect(screen.getAllByTitle('sensitivity_label: internal')).toHaveLength(2);
  expect(screen.getAllByTitle('sensitivity_label: critical')).toHaveLength(2);
  expect(screen.queryByTitle('sensitivity_label: unknown')).not.toBeInTheDocument();
});
