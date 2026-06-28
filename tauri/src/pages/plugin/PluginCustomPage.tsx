import { useParams } from 'react-router-dom';
import { WatermarkPluginPage } from './WatermarkPluginPage';

const CUSTOM_UI_REGISTRY: Record<string, React.ComponentType> = {
  WatermarkPluginPage,
};

export function PluginCustomPage() {
  const { pluginId } = useParams<{ pluginId: string }>();
  // 实际组件由插件 manifest 的 customUi 字段决定；这里简化处理：
  // 只有水印插件使用此路由，其余按 customUi 注册表分发。
  const Component = pluginId ? CUSTOM_UI_REGISTRY[pluginId] : undefined;
  if (!Component) {
    return (
      <div style={{ padding: 24 }}>
        未找到插件自定义页面: {pluginId}
      </div>
    );
  }
  return <Component />;
}
