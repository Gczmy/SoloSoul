import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import type { ObjectSummary } from '@/stores/objectStore';
import { ObjectRulerPreview } from './ObjectRulerPreview';

const object: ObjectSummary = {
  id: 'preview',
  name: '旅行证件',
  typeId: 'travel',
  sensitivityLevel: 'internal',
  createdAt: '',
  updatedAt: '',
  properties: {
    destination: '京都',
    passport: 'SECRET-PASSPORT-123',
    note: 'SECRET-NOTE-456',
    __token: 'INTERNAL-METADATA',
  },
  propertyLabels: { destination: 'public', passport: 'critical' },
};

describe('对象尺标简介', () => {
  it('只显示公开字段明文，未知敏感度默认掩码，内部元数据不进入简介', () => {
    const onNavigate = vi.fn();
    render(
      <ObjectRulerPreview
        object={object}
        collectionLabel="旅行"
        index={1}
        total={60}
        onNavigate={onNavigate}
      />,
    );
    expect(screen.getByText('京都')).toBeInTheDocument();
    expect(screen.getAllByText('••••••••')).toHaveLength(2);
    expect(document.body.textContent).not.toContain('SECRET-');
    expect(document.body.innerHTML).not.toContain('SECRET-');
    expect(document.body.textContent).not.toContain('INTERNAL-METADATA');
    fireEvent.click(screen.getByRole('button', { name: 'object_ruler_jump' }));
    expect(onNavigate).toHaveBeenCalledOnce();
  });
  it.each([null, 'invalid', { nested: 'public' }])(
    '非法显式标签不得回退到模板公开字段：%s',
    (label) => {
      render(
        <ObjectRulerPreview
          object={{
            ...object,
            properties: { secret: 'RF302-PRIVATE' },
            propertyLabels: { secret: label },
          }}
          template={{
            id: 'template',
            accountId: 'account',
            name: 'Template',
            createdAt: '',
            properties: [
              { id: 'secret', name: 'Secret', type: 'text', sensitivityLevel: 'public' },
            ],
          }}
          collectionLabel="合成"
          index={0}
          total={1}
          onNavigate={() => {}}
        />,
      );
      expect(screen.getByText('••••••••')).toBeInTheDocument();
      expect(document.body.innerHTML).not.toContain('RF302-PRIVATE');
    },
  );
});
