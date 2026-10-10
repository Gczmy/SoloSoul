// RF-121: fixed read-only surface state, no text, business data or arbitrary selectors.
(() => {
  const { runId } = __REQUEST__;
  const root = document.documentElement;
  const color = (value) => {
    const s = value.trim().toLowerCase();
    if (/^#[0-9a-f]{6}$/.test(s))
      return [1, 3, 5].map((i) => parseInt(s.slice(i, i + 2), 16)).concat(1);
    if (/^#[0-9a-f]{3}$/.test(s)) return [1, 2, 3].map((i) => parseInt(s[i] + s[i], 16)).concat(1);
    const rgb = s.match(/^rgba?\(([^)]+)\)$/);
    if (rgb) {
      const values = rgb[1]
        .split(/[\s,/]+/)
        .filter(Boolean)
        .map(Number);
      if (values.length === 3) values.push(1);
      if (values.length === 4 && values.every(Number.isFinite)) return values;
    }
    const srgb = s.match(/^color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\)$/);
    if (srgb)
      return [
        Number(srgb[1]) * 255,
        Number(srgb[2]) * 255,
        Number(srgb[3]) * 255,
        Number(srgb[4] ?? 1),
      ];
    return null;
  };
  const one = (selector) => {
    const nodes = document.querySelectorAll(selector);
    return nodes.length === 1 ? nodes[0] : null;
  };
  const navigation = one('#desktop-navigation');
  const surface = (node) => {
    if (!node) return null;
    const style = getComputedStyle(node),
      r = node.getBoundingClientRect();
    if (style.visibility !== 'visible' || style.display === 'none') return null;
    return {
      rect: [r.x, r.y, r.width, r.height],
      background: color(style.backgroundColor),
      text: color(style.color),
      radius: Number.parseFloat(style.borderTopLeftRadius),
    };
  };
  const style = getComputedStyle(root);
  const bool = (value) => (value === 'true' ? true : value === 'false' ? false : null);
  return {
    schemaVersion: 1,
    scope: 'windows-native-surface-state',
    runId,
    href: location.href,
    timeOriginMs: performance.timeOrigin,
    atMs: performance.now(),
    theme: root.dataset.theme,
    platform: root.dataset.desktopPlatform,
    material: root.dataset.nativeMaterial,
    highContrast: bool(root.dataset.highContrast),
    forcedColors: matchMedia('(forced-colors: active)').matches,
    reducedTransparency: matchMedia('(prefers-reduced-transparency: reduce)').matches,
    focused: document.hasFocus(),
    visibility: document.visibilityState,
    viewport: [innerWidth, innerHeight, devicePixelRatio],
    expanded: bool(navigation?.getAttribute('data-expanded')),
    palette: {
      base: color(style.getPropertyValue('--bg-base')),
      elevated: color(style.getPropertyValue('--bg-elevated')),
      text: color(style.getPropertyValue('--text-primary')),
    },
    surfaces: {
      navigation: surface(navigation),
      appbar: surface(one('[data-appbar]')),
      content: surface(one('[data-shell-content]')),
      card: surface(document.querySelector('[data-shell-content] [data-ui-card]')),
    },
  };
})();
