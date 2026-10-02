import fixtures from './platformCapabilities.json';
import type { PlatformCapabilities } from '../generated/ipcContracts';

/** 固定期望契约，生产代码不从本 fixture 推导平台能力。 */
export const capabilityFixture = (os: string): PlatformCapabilities =>
  structuredClone(fixtures[os as keyof typeof fixtures]) as PlatformCapabilities;
