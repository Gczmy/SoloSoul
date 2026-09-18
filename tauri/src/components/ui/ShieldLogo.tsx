import type { CSSProperties } from 'react';
import appIcon from '../../../src-tauri/icons/128x128@2x.png';

/** 与桌面安装包共用图标资源，不随账户主题改变品牌形状或颜色。 */
export function ShieldLogo({ size = 32, style }: { size?: number; style?: CSSProperties }) {
  return (
    <img
      src={appIcon}
      alt="SoloSoul"
      width={size}
      height={size}
      style={{
        display: 'block',
        width: size,
        height: size,
        flexShrink: 0,
        objectFit: 'contain',
        ...style,
      }}
    />
  );
}
