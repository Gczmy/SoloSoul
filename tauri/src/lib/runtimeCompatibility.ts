/** Android 12 自带的旧 WebView 仍可加载应用；仅补齐业务已使用的两个标准 API。 */
export function installRuntimeCompatibility(
  objectApi: typeof Object = Object,
  cryptoApi: Crypto | undefined = globalThis.crypto,
) {
  if (typeof objectApi.hasOwn !== 'function') {
    Object.defineProperty(objectApi, 'hasOwn', {
      configurable: true,
      writable: true,
      value: (value: object, key: PropertyKey) => Object.prototype.hasOwnProperty.call(value, key),
    });
  }
  if (cryptoApi && typeof cryptoApi.randomUUID !== 'function') {
    Object.defineProperty(cryptoApi, 'randomUUID', {
      configurable: true,
      writable: true,
      value: () => {
        // 保持 UUID v4 的安全随机来源，不以时间戳或 Math.random 替代。
        const bytes = cryptoApi.getRandomValues(new Uint8Array(16));
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
        return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
      },
    });
  }
}
