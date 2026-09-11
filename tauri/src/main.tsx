import type {} from '@/lib/startupScreen';
import { prepareStartupWindow } from '@/lib/nativeWindow';

void prepareStartupWindow();

void import('./bootstrapApp')
  .then(({ mountApplication }) => mountApplication())
  .catch(() => window.__SOLOSOUL_STARTUP__?.fail('initialization-failed'));
