// 通过实际 Host 契约检索指南；账户与会话均由本次请求起点提供。
import { createSessionRequests } from '@/lib/sessionRequests';
import type { IpcCommands } from '@/lib/generated/ipcContracts';
export type GuideChunk = IpcCommands['llm_search_guide_chunks']['result'][number];
export type GuideSearchRequest = Pick<
  ReturnType<ReturnType<typeof createSessionRequests>['begin']>,
  'assertCurrent' | 'invokeTyped'
>;
const requests = createSessionRequests();
/** Embedding 不可用时 Host 继续回退关键词检索；失效会话不能接纳任何片段。 */
export async function searchGuideChunks(
  accountId: string,
  query: string,
  language: string,
  request?: GuideSearchRequest,
  topK = 3,
): Promise<GuideChunk[]> {
  const ticket = request ?? requests.begin(undefined, accountId);
  ticket.assertCurrent();
  try {
    return await ticket.invokeTyped('llm_search_guide_chunks', {
      accountId,
      query,
      language,
      topK,
    });
  } catch {
    ticket.assertCurrent();
    // 保留原检索不可用时的空上下文回退，不把旧会话的失败吞成新会话结果。
    return [];
  }
}
