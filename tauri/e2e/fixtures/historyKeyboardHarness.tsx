import { createRoot } from 'react-dom/client';
import { HistoryViewer } from '../../src/components/object/HistoryViewer';

/** Vite 测试夹具：挂载真实组件以验证浏览器原生按钮键盘行为。 */
export function mount() {
  const container = document.createElement('div');
  container.id = 'history-keyboard-fixture';
  document.body.append(container);
  createRoot(container).render(
    <HistoryViewer
      objectId="keyboard-fixture"
      onClose={() => {}}
      passwordVerify={async () => ({ ok: true, method: 'password' })}
      getFieldSensitivity={() => 'sensitive'}
      isFieldDeprecated={() => false}
      getFieldName={(key) => key}
    />,
  );
}
