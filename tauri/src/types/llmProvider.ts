/**
 * LLM Provider 配置（P037 收敛单一来源）。
 * 此前 ProviderManagerPanel.tsx 与 LlmStatsPage.tsx 各定义一份。
 */
export interface ProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  isEnabled: boolean;
  isBuiltIn: boolean;
  apiKey: string;
  apiType: 'openAI' | 'anthropic';
  /** P021: 嵌入模型名（Rust ProviderConfig/ProviderWithKey 的 embedding_model）。
   * 此前 TS 缺此字段，重构保存逻辑时会静默重置该值。 */
  embeddingModel?: string;
}
