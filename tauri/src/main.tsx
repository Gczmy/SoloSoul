import type {} from '@/lib/startupScreen';

void import('./bootstrapApp')
  .then(({ mountApplication }) => mountApplication())
  .catch(() => window.__SOLOSOUL_STARTUP__?.fail('initialization-failed'));
