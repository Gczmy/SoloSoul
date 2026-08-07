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
}
