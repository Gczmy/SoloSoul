/**
 * 判断当前是否处于开发或调试模式。
 * 发布版本（production build）返回 false。
 */
export function isDevOrDebug(): boolean {
  return (
    import.meta.env.DEV === true ||
    import.meta.env.MODE === 'debug' ||
    import.meta.env.VITE_SOLOSOUL_DEBUG === 'true'
  );
}
