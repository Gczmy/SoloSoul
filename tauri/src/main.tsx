import type {} from '@/lib/startupScreen';
import { installRuntimeCompatibility } from '@/lib/runtimeCompatibility';
import { prepareStartupWindow } from '@/lib/nativeWindow';

installRuntimeCompatibility();
void prepareStartupWindow();

void import('./bootstrapApp')
  .then(({ mountApplication }) => mountApplication())
  .catch(() => window.__SOLOSOUL_STARTUP__?.fail('initialization-failed'));
