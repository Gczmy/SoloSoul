import { createContext, useContext, type ReactNode } from 'react';
import styles from './ShellNotifications.module.css';

const ShellNotificationsContext = createContext<ReactNode>(null);

/** 状态仍由 AppRoutes 持有，通知交给当前布局呈现，避免创建浮在正文上方的全局层。 */
export function ShellNotificationsProvider({
  notifications,
  children,
}: {
  notifications: ReactNode;
  children: ReactNode;
}) {
  return (
    <ShellNotificationsContext.Provider value={notifications}>
      {children}
    </ShellNotificationsContext.Provider>
  );
}

/** 登录和已登录壳共用正常流通知槽；无通知时高度自然归零。 */
export function ShellNotificationSlot() {
  const notifications = useContext(ShellNotificationsContext);
  return (
    <div className={styles.slot} data-shell-notifications>
      {notifications}
    </div>
  );
}
