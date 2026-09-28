/**
 * 编译型契约回归：由项目 tsc --noEmit 检查，函数不会在运行时调用。
 * 如果错误调用被宽泛重载放行，未使用的 @ts-expect-error 将使 tsc 失败。
 */
import type { AppInfo } from './generated/ipcContracts';
import type { InvokeOptions } from './ipcClient';
import { invokeTypedCommand } from './typedIpc';

export async function assertTypedIpcContract(dynamicCommand: string, options: InvokeOptions) {
  const info = await invokeTypedCommand('get_app_info');
  const compatible: AppInfo = info;
  const name: string = info.appName;
  const version: string = info.version;
  const os: string = info.os;
  const arch: string = info.arch;
  await invokeTypedCommand('get_app_info', undefined, options);
  const completeCall = ['get_app_info', undefined, options] as const;
  const tupleResult: AppInfo = await invokeTypedCommand(...completeCall);
  void tupleResult;
  await invokeTypedCommand('get_app_info', undefined, {
    requireUnlocked: true,
    requestIsCurrent: () => true,
  });

  // @ts-expect-error 未登记命令不能回退任意字符串入口。
  void invokeTypedCommand('get_app_info_typo');
  // @ts-expect-error 宽 string 不能绕开已登记命令集合。
  void invokeTypedCommand(dynamicCommand);
  // @ts-expect-error 无参数命令不接受一个看似空的对象。
  void invokeTypedCommand('get_app_info', {});
  const wrongArgs = { accountId: 'synthetic-account' };
  // @ts-expect-error 变量参数也不能反推宽化命令类型。
  void invokeTypedCommand('get_app_info', wrongArgs);
  // @ts-expect-error 无参数不等于 null 参数。
  void invokeTypedCommand('get_app_info', null);
  // @ts-expect-error 选项仍在第三参，不能混入参数位置。
  void invokeTypedCommand('get_app_info', { requireUnlocked: false });
  // @ts-expect-error 选项保留原 InvokeOptions 的 boolean 类型。
  void invokeTypedCommand('get_app_info', undefined, { requireUnlocked: 'yes' });
  // @ts-expect-error 会话检查必须返回 boolean。
  void invokeTypedCommand('get_app_info', undefined, { requestIsCurrent: () => 'current' });
  // @ts-expect-error 调用者不能用泛型指定自己希望的响应类型。
  void invokeTypedCommand<AppInfo>('get_app_info');
  // @ts-expect-error 显式泛型也没有任意 string 命令兜底。
  void invokeTypedCommand<string>('get_app_info');
  // @ts-expect-error 返回值由命令映射决定，而不是上下文指定的 string。
  const wrongPromise: Promise<string> = invokeTypedCommand('get_app_info');
  // @ts-expect-error Rust version 字段是必需字符串。
  const wrongVersion: number = info.version;
  // @ts-expect-error 序列化字段为 camelCase，不接受 app_name。
  const snakeCaseName = info.app_name;
  // @ts-expect-error 生成 DTO 的 arch 必需，不能静默变为可选。
  const incomplete: AppInfo = { appName: 'SoloSoul', version: '1.2.3', os: 'windows' };

  void wrongPromise;
  void wrongVersion;
  void snakeCaseName;
  void incomplete;
  return { compatible, name, version, os, arch };
}
