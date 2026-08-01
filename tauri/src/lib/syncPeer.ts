/**
 * 设备显示名格式化：fingerprint 非空 → SoloSoul-<fp 前 8 位>，否则回退 node_id。
 *
 * 与后端 record_peer（session.rs）新记录存 `SoloSoul-<fp8>`、QR 卡片设备名
 * （sync.rs:854-858）、移动端 NSD 注册名的规则保持一致。
 * 已知设备列表、配对对话框、发现列表统一使用，保证跨设备名称一致性。
 */
export function formatPeerName(peer: {
  id: string;
  name?: string;
  fingerprint?: string;
}): string {
  if (peer.fingerprint && peer.fingerprint.length > 0) {
    return `SoloSoul-${peer.fingerprint.slice(0, 8)}`;
  }
  return peer.name || peer.id;
}
