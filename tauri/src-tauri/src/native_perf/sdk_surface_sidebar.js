// Fixed read-only homepage probe. Real input is sent by the native SDK, never by JS.
(async () => {
  const navigation = document.getElementById('desktop-navigation');
  const root = document.documentElement;
  const rect = (node) => {
    if (!node) return null;
    const r = node.getBoundingClientRect();
    return [r.x, r.y, r.width, r.height];
  };
  const layout = () => [
    rect(navigation),
    rect(document.querySelector('[data-appbar]')),
    rect(document.querySelector('[data-shell-content]')),
  ];
  // Observe animation settling; do not change styles or suppress animations.
  const began = performance.now();
  let previous = '',
    identical = 0;
  while (performance.now() - began < 2000) {
    await new Promise(requestAnimationFrame);
    const current = JSON.stringify(layout());
    identical = current === previous ? identical + 1 : 0;
    previous = current;
    if (performance.now() - began >= 350 && identical >= 3) break;
  }
  const expanded = navigation?.getAttribute('data-expanded');
  const label = expanded === 'true' ? 'Collapse sidebar' : 'Expand sidebar';
  const buttons = [
    ...(navigation?.querySelectorAll('button[aria-controls="desktop-navigation"]') ?? []),
  ].filter((node) => node.getAttribute('aria-label') === label);
  const node = buttons.length === 1 ? buttons[0] : null;
  const r = node?.getBoundingClientRect();
  const x = r ? r.x + r.width / 2 : null,
    y = r ? r.y + r.height / 2 : null;
  const hit = r ? document.elementFromPoint(x, y) : null;
  const style = node ? getComputedStyle(node) : null;
  return {
    schemaVersion: 1,
    scope: 'windows-native-sidebar-probe',
    href: location.href,
    timeOriginMs: performance.timeOrigin,
    atMs: performance.now(),
    theme: root.dataset.theme,
    platform: root.dataset.desktopPlatform,
    expanded: expanded === 'true' ? true : expanded === 'false' ? false : null,
    viewport: [innerWidth, innerHeight, devicePixelRatio],
    focused: document.hasFocus(),
    visibility: document.visibilityState,
    layout: layout(),
    settled: identical >= 3 && performance.now() - began >= 350,
    target: {
      x,
      y,
      actionable: !!(
        r &&
        r.width > 0 &&
        r.height > 0 &&
        x >= 0 &&
        y >= 0 &&
        x < innerWidth &&
        y < innerHeight &&
        style.visibility === 'visible' &&
        style.display !== 'none' &&
        !node.disabled &&
        !node.closest('[inert]') &&
        (hit === node || node.contains(hit))
      ),
    },
    inputTrust: window.__SOLOSOUL_SDK_JOURNEY_INPUTS__?.snapshot(),
  };
})();
