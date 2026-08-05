/**
 * 设备显示名格式化：fingerprint 非空 → SoloSoul-<fp 前 8 位>，否则回退 device name。
 *
 * 与后端 record_peer（session.rs）新记录存 `SoloSoul-<fp8>`、QR 卡片设备名
 * （sync.rs:854-858）、移动端 NSD 注册名的规则保持一致。
 * 已知设备列表、配对对话框、发现列表统一使用，保证跨设备名称一致性。
 *
 * 旧记录/未配对 peer 的 peer_name 可能是原始 node_id（`node_<uuid>`，32 位 hex，
 * 远超安卓端卡片宽度）——此类名称在前端裁剪为短形式展示（`node_<前 8 位>…`），
 * 避免设备名溢出卡片。已配对的 peer 因有指纹恒走 `SoloSoul-<fp8>` 分支，不受影响。
 */
export function formatPeerName(peer: {
  id: string;
  name?: string;
  fingerprint?: string;
}): string {
  if (peer.fingerprint && peer.fingerprint.length > 0) {
    return `SoloSoul-${peer.fingerprint.slice(0, 8)}`;
  }
  const raw = peer.name || peer.id || 'Unknown device';
  // node_<uuid>（node_ + 32 位 hex）或纯 32 位 hex 的原始 ID：裁剪展示
  if (/^node_[0-9a-f]{8,}$/i.test(raw) || /^[0-9a-f]{32}$/i.test(raw)) {
    return `${raw.slice(0, 13)}…`;
  }
  return raw;
}

/**
 * 已发现设备显示名格式化：剥掉 mDNS 全名后缀（`._solosoul._tcp.local.` /
 * `._solosoul_recovery._tcp.local.` / `.local.`），长原始 ID（`node_<uuid>` /
 * 32 位 hex）裁剪展示（与 formatPeerName 同一规则）。
 *
 * 新版对端广播友好实例名 `SoloSoul-<fp8>`，本函数原样保留；旧版对端仍广播
 * `node_<uuid>` 实例名时，裁剪为 `node_<前 8 位>…`，配合 CSS ellipsis 双重
 * 保证设备名不溢出卡片（安卓端宽屏受限场景）。
 */
export function formatDiscoveredName(device: { name?: string }): string {
  let raw = (device.name || '').trim();
  if (!raw) return 'Unknown device';
  raw = raw
    .replace(/\._solosoul(?:_recovery)?\._tcp\.local\.$/i, '')
    .replace(/\.local\.$/i, '');
  if (/^node_[0-9a-f]{8,}$/i.test(raw) || /^[0-9a-f]{32}$/i.test(raw)) {
    return `${raw.slice(0, 13)}…`;
  }
  return raw;
}
