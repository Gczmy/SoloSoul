import { useState } from 'react';
import { SampleTemplateGallery } from '@/components/template/SampleTemplateGallery';
import { SampleTemplateDetail } from '@/components/template/SampleTemplateDetail';
import type { SampleTemplate } from '@/lib/sampleTemplates';

interface SampleGallerySectionProps {
  /** 由编排层 AppShell 动作按钮控制（P224-③ 拆分） */
  isOpen: boolean;
  onClose: () => void;
  /**
   * 从示例模板创建：编排层实现（推导 contractBindings + createTemplate + toast），
   * 失败时上抛以保持详情页打开（与拆分前行为一致）。
   */
  onUseSample: (tpl: SampleTemplate) => Promise<void>;
}

/**
 * 示例模板画廊 + 详情（纯展示，P224-③ 拆分）：
 * selectedSample 为本组件内部状态（仅被自身详情视图消费）。
 */
export function SampleGallerySection({ isOpen, onClose, onUseSample }: SampleGallerySectionProps) {
  const [selectedSample, setSelectedSample] = useState<SampleTemplate | null>(null);

  return (
    <>
      <SampleTemplateGallery
        isOpen={isOpen}
        onClose={onClose}
        onSelect={(tpl) => {
          setSelectedSample(tpl);
        }}
      />
      {selectedSample && (
        <SampleTemplateDetail
          template={selectedSample}
          onBack={() => setSelectedSample(null)}
          onUse={async () => {
            if (!selectedSample) return;
            try {
              await onUseSample(selectedSample);
              setSelectedSample(null);
            } catch {
              /* 创建失败已由编排层 toast，保持详情打开（与拆分前一致） */
            }
          }}
        />
      )}
    </>
  );
}
