import { createContext } from 'react';

/** 仅垂直侧栏提供展开状态，顶部/底部导航和弹层内其他按钮保持原布局。 */
export const DesktopSidebarContext = createContext(false);
