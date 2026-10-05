/** RF-312：结构诊断的字段与预算；结构存在不等于加载或渲染完成。 */
const exact = (v, keys) =>
  v &&
  typeof v === 'object' &&
  !Array.isArray(v) &&
  Object.keys(v).length === keys.length &&
  keys.every((k) => Object.hasOwn(v, k));
const number = (v, max) => Number.isSafeInteger(v) && v >= 0 && v <= max;
const id = (v) => typeof v === 'string' && /^[A-Za-z0-9_.:-]{1,128}$/.test(v);
const classes = ['application', 'owned-pdf', 'component-extension', 'blank', 'other'];
const geometry = (v, embed) =>
  exact(
    v,
    embed ? ['kind', 'sourceClass', 'visible', 'width', 'height'] : ['visible', 'width', 'height'],
  ) &&
  typeof v.visible === 'boolean' &&
  number(v.width, 16384) &&
  number(v.height, 16384) &&
  (!embed ||
    (['iframe', 'embed', 'object'].includes(v.kind) &&
      ['owned-pdf', 'component-extension', 'blank', 'other'].includes(v.sourceClass)));
export function pdfStructureValid(v) {
  if (
    !exact(v, [
      'schemaVersion',
      'scope',
      'documentReadyState',
      'visibilityState',
      'scannedElements',
      'openShadowRoots',
      'truncated',
      'counts',
      'customTags',
      'embeds',
      'canvases',
    ]) ||
    v.schemaVersion !== 1 ||
    v.scope !== 'windows-native-sdk-pdf-structure' ||
    !['loading', 'interactive', 'complete'].includes(v.documentReadyState) ||
    !['visible', 'hidden'].includes(v.visibilityState) ||
    !number(v.scannedElements, 512) ||
    !number(v.openShadowRoots, 8) ||
    v.openShadowRoots > v.scannedElements ||
    typeof v.truncated !== 'boolean'
  )
    return false;
  const c = v.counts,
    kinds = ['embed', 'object', 'iframe', 'canvas', 'pdfViewer', 'customElements'];
  if (
    !exact(c, kinds) ||
    kinds.some((k) => !number(c[k], v.scannedElements)) ||
    c.embed + c.object + c.iframe + c.canvas > v.scannedElements ||
    c.pdfViewer > c.customElements
  )
    return false;
  if (
    !Array.isArray(v.customTags) ||
    v.customTags.length > 32 ||
    v.customTags.length > c.customElements ||
    new Set(v.customTags).size !== v.customTags.length ||
    v.customTags.some(
      (t) => typeof t !== 'string' || !t.includes('-') || !/^[a-z][a-z0-9-]{0,63}$/.test(t),
    )
  )
    return false;
  return (
    Array.isArray(v.embeds) &&
    v.embeds.length <= 8 &&
    v.embeds.length <= c.embed + c.object + c.iframe &&
    v.embeds.every((e) => geometry(e, true)) &&
    Array.isArray(v.canvases) &&
    v.canvases.length <= 8 &&
    v.canvases.length <= c.canvas &&
    v.canvases.every((c) => geometry(c, false))
  );
}
export function pdfFrameTreesValid(v, root) {
  if (!exact(v, ['before', 'after'])) return false;
  for (const tree of [v.before, v.after]) {
    if (!Array.isArray(tree) || tree.length === 0 || tree.length > 8) return false;
    const seen = new Set(),
      depth = new Map();
    for (let i = 0; i < tree.length; i++) {
      const f = tree[i];
      if (
        !exact(f, ['id', 'parentId', 'loaderId', 'urlClass']) ||
        !id(f.id) ||
        seen.has(f.id) ||
        !(f.loaderId === null || id(f.loaderId)) ||
        !classes.includes(f.urlClass)
      )
        return false;
      if (
        i === 0
          ? f.parentId !== null || ['id', 'loaderId', 'urlClass'].some((k) => f[k] !== root[k])
          : !seen.has(f.parentId)
      )
        return false;
      const level = i === 0 ? 0 : depth.get(f.parentId) + 1;
      if (level > 3) return false;
      depth.set(f.id, level);
      seen.add(f.id);
    }
  }
  return true;
}
