/**
 * 轻量前端日志工具
 *
 * - logger.warn → 仅在开发/调试模式下输出，减少生产环境控制台噪音。
 * - logger.error → 始终输出（生产环境也保留），用于真正需要追溯的异常。
 *
 * 后续可在此处统一接入后端 log_write IPC、远端日志收集等基础设施，
 * 调用方无需逐个修改。
 *
 * 用法：
 *   import { logger } from '@/lib/logger';
 *   logger.warn('[Component] ...:', err);
 *   logger.error('[Component] ...:', err);
 */

import { isDevOrDebug } from './utils';

export const logger = {
  /**
   * 非关键性异常日志。仅在开发/调试模式下输出到 console.warn。
   * 适用于 .catch() 中可优雅降级的背景操作失败。
   */
  warn: (...args: unknown[]): void => {
    if (isDevOrDebug()) {
      console.warn(...args);
    }
  },

  /**
   * 关键性异常日志。所有构建模式均输出到 console.error。
   * 适用于安全操作失败（如自动锁定）、存储层故障等需要追溯的问题。
   */
  error: (...args: unknown[]): void => {
    console.error(...args);
  },
};
