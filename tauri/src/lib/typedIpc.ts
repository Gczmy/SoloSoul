/** 已迁移命令的严格类型入口；未迁移命令继续使用 ipcClient。 */
import type { IpcCommands } from './generated/ipcContracts';
import { invokeCommand, type InvokeOptions } from './ipcClient';

type CommandCall = {
  [C in keyof IpcCommands]: IpcCommands[C]['args'] extends undefined
    ? [command: C, args?: undefined, options?: InvokeOptions]
    : [command: C, args: IpcCommands[C]['args'], options?: InvokeOptions];
}[keyof IpcCommands];

// 泛型推断会接受结构子类型；单独拒绝多余参数键，防止误传另一个命令的参数。
type ExactArguments<Call extends CommandCall> = Call extends CommandCall
  ? Exclude<
      keyof NonNullable<Call[1]>,
      keyof NonNullable<IpcCommands[Call[0]]['args']>
    > extends never
    ? unknown
    : never
  : never;

/** 命令和参数共享判别元组；会话入口复用同一签名和既有传输守卫。 */
export function createTypedInvoker(transport: typeof invokeCommand) {
  return function invokeTyped<Call extends CommandCall>(
    ...call: Call & ExactArguments<NoInfer<Call>>
  ): Promise<IpcCommands[Call[0]]['result']> {
    const [command, args, options] = call;
    return transport<IpcCommands[Call[0]]['result']>(command, args, options);
  };
}

export const invokeTypedCommand = createTypedInvoker(invokeCommand);
