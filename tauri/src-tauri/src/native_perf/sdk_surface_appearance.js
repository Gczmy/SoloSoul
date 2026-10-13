// Fixed read-only scene probe. Only the native SDK sends real input.
(async () => {
  const spec = __SCENE_PROBE__;
  const root = document.documentElement;
  const nav = document.getElementById('desktop-navigation');
  const home = 'http://tauri.localhost/';
  const settings = home + 'settings';
  const appearance = settings + '/appearance';
  const one = (nodes) => (nodes.length === 1 ? nodes[0] : null);
  const button = (label) =>
    one(
      [...(nav?.querySelectorAll('button') ?? [])].filter(
        (n) => n.getAttribute('aria-label') === label,
      ),
    );
  const rectangle = (node) => {
    if (!node) return null;
    const r = node.getBoundingClientRect();
    return [r.x, r.y, r.width, r.height];
  };
  const preferences = ['light', 'dark', 'system'];
  const read = () => {
    const inputs = [...document.querySelectorAll('input[type="radio"][name="theme"]')];
    const labels = inputs.map((n) => n.closest('label')?.textContent.trim() ?? '');
    const radiosValid =
      inputs.length === 3 &&
      labels[0].startsWith('Light · ') &&
      labels[1].startsWith('Dark · ') &&
      labels[2] === 'System';
    const checked =
      radiosValid && inputs.filter((n) => n.checked).length === 1
        ? preferences[inputs.findIndex((n) => n.checked)]
        : null;
    const expanded = nav?.getAttribute('data-expanded') === 'true';
    let action = spec.action;
    if (action === 'recover') {
      if (location.href === home)
        action = !expanded
          ? 'toggle'
          : spec.verified || spec.original === null
            ? 'done'
            : 'settings';
      else if (location.href === settings) action = 'appearance';
      else if (location.href === appearance)
        action = spec.original === null || checked === spec.original ? 'home' : 'original';
      else action = 'invalid';
    }
    let node = null;
    if (action === 'settings') node = button('Settings');
    if (action === 'home') node = button('Home');
    if (action === 'appearance')
      node = one(
        [...document.querySelectorAll('[data-ui-card][role="button"]')].filter((n) =>
          n.textContent.trim().startsWith('Theme & Appearance'),
        ),
      );
    if (action === 'dark' || action === 'original') {
      const preference = action === 'dark' ? 'dark' : spec.original;
      node = radiosValid ? inputs[preferences.indexOf(preference)] : null;
    }
    if (action === 'toggle')
      node = one(
        [...(nav?.querySelectorAll('button[aria-controls="desktop-navigation"]') ?? [])].filter(
          (n) =>
            n.getAttribute('aria-label') === (expanded ? 'Collapse sidebar' : 'Expand sidebar'),
        ),
      );
    const r = node?.getBoundingClientRect();
    const x = r ? r.x + r.width / 2 : null,
      y = r ? r.y + r.height / 2 : null;
    const hit = r ? document.elementFromPoint(x, y) : null;
    const style = node ? getComputedStyle(node) : null;
    const actionable = !!(
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
    );
    const route = ['settings', 'toggle', 'done'].includes(action)
      ? home
      : action === 'appearance'
        ? settings
        : appearance;
    const expectedTheme =
      spec.theme ??
      (action === 'done' || (action === 'home' && spec.action === 'recover') ? 'light' : null);
    const ready =
      location.href === route &&
      ['light', 'dark'].includes(root.dataset.theme) &&
      (expectedTheme === null || root.dataset.theme === expectedTheme) &&
      (location.href !== appearance || radiosValid) &&
      (spec.checked === null || checked === spec.checked) &&
      (action === 'done' || actionable);
    return {
      action,
      checked,
      radios: radiosValid ? inputs.map((n) => n.checked) : [],
      expanded:
        nav?.getAttribute('data-expanded') === 'true'
          ? true
          : nav?.getAttribute('data-expanded') === 'false'
            ? false
            : null,
      layout: [
        rectangle(nav),
        rectangle(document.querySelector('[data-appbar]')),
        rectangle(document.querySelector('[data-shell-content]')),
      ],
      target: action === 'done' ? null : { x, y, actionable },
      ready,
    };
  };
  const began = performance.now();
  let previous = '',
    identical = 0,
    data;
  while (performance.now() - began < 2000) {
    await new Promise(requestAnimationFrame);
    data = read();
    const key = JSON.stringify([location.href, root.dataset.theme, data]);
    identical = key === previous ? identical + 1 : 0;
    previous = key;
    if (performance.now() - began >= 350 && identical >= 3 && data.ready) break;
  }
  return {
    schemaVersion: 1,
    scope: 'windows-native-appearance-probe',
    action: data.action,
    href: location.href,
    timeOriginMs: performance.timeOrigin,
    atMs: performance.now(),
    theme: root.dataset.theme,
    platform: root.dataset.desktopPlatform,
    expanded: data.expanded,
    viewport: [innerWidth, innerHeight, devicePixelRatio],
    focused: document.hasFocus(),
    visibility: document.visibilityState,
    layout: data.layout,
    settled: identical >= 3 && performance.now() - began >= 350 && data.ready,
    target: data.target,
    checked: data.checked,
    radios: data.radios,
    inputTrust: window.__SOLOSOUL_SDK_JOURNEY_INPUTS__?.snapshot(),
    observer: window.__SOLOSOUL_NATIVE_PERF__?.snapshot(),
  };
})();
