/** 已迁移命令的严格类型入口；未迁移命令继续使用 ipcClient。 */
import type { IpcCommands } from './generated/ipcContracts';
import { invokeCommand, type InvokeOptions } from './ipcClient';

type CommandCall = {
  [C in keyof IpcCommands]: IpcCommands[C]['args'] extends undefined
    ? [command: C, args?: undefined, options?: InvokeOptions]
    : [command: C, args: IpcCommands[C]['args'], options?: InvokeOptions];
}[keyof IpcCommands];

/** 命令和参数共享判别元组；联合命令必须先缩窄，不能接受另一命令的参数。 */
export function invokeTypedCommand<Call extends CommandCall>(
  ...[command, args, options]: Call
): Promise<IpcCommands[Call[0]]['result']> {
  // 守卫、过期请求检查、单参原生调用与原始错误都继续由同一传输层处理。
  return invokeCommand<IpcCommands[Call[0]]['result']>(command, args, options);
}
