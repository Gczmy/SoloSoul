# 自有更新分发接入指南

本实现支持将签名后的公开更新包镜像到自有对象存储/CDN。仓库没有预设域名，
`tauri/src-tauri/update-sources.json` 默认为空；此时保留现有 GitHub/代理来源。
当前代码变更不会创建账号、购买服务或上传任何资产。

桌面与 Android 共用以下下载行为：

- 先读取少量真实包数据测速；首选源速度正常时直接下载，低速或失败才并发探测备选源。
- 下载中停流或持续低速会切换线路，使用 Range 接续已下载部分。若所有线路都慢，
  最后一轮允许持续低速完成，仍对停流设置超时。
- 更新横幅和关于页面显示当前线路、速度与切换状态。取消后退回更新选项，保留当前包的
  断点；下次重试或重启客户端后可继续。新版本会清理旧版断点。
- 服务器不支持续传时从头覆盖，绝不把完整包追加到旧前缀。桌面完成后验证原 updater
  签名，Android 验证已签名的 SHA-256；失败删除无效缓存并拒绝安装。

下载包目前限制为 512 MiB。线路探测改善失效代理与持续低速时的恢复；没有可靠的自有
分发源时，实际速度仍受 GitHub、代理及用户网络限制，不能保证国内各网络都能高速下载。

## 更新检查日志与来源偏好

- 桌面 updater 清单、Release 备用通道及 Android 成功读取版本后，输出 `INFO`：
  获取通道、来源主机、当前版本、实际读取版本及是否发现新版本。单条来源的错误不代表
  最终检查失败，可用后续成功日志确认结果；日志不输出账户信息或带参数的下载链接。
- 记住成功的清单来源（updater / Release 分开保存），下次给优先源 900ms 的独占窗口。
  响应正常时直接使用；失败立即探测其他来源，较慢时保留在途请求并启动其他候选。
  桌面上次通过 Release 备用通道成功时，下次也会先尝试该通道。
- 优先结果低于本机版本时仍探测其他源；每 6 小时重新进行完整探测，快速成功不延长
  该周期，减少长期使用陈旧镜像的风险。仍只从当前允许的候选中选择；关闭代理或删除
  配置后，旧偏好不会重新引入已禁用来源。
- 已解锁时写入账户加密 `preferences.updateSources`，同时保留 `ui_preferences.json`
  中不含账户信息的本机缓存，供登录前检查和重启恢复。尚未解锁时只写缓存，在之后的
  已解锁检查成功时写入账户。请求期间切换账户不会把结果写入新账户；快速路径的来源未变化时
  不反复推进 Profile 版本。偏好只影响版本元数据请求，大包下载仍独立测速并严格验签。

更新说明在横幅、关于页和强制更新界面共用 GFM 渲染器。横幅打开时不再额外加载一个
桥接模块，也不在失败后静默退回 Markdown 原文；表格区域可独立滚动和键盘聚焦。
`e2e/release-notes.production.spec.ts` 在生产构建下验证三个入口及长文件名、小屏边界。

## 1. 配置编译期更新源

配置文件：`tauri/src-tauri/update-sources.json`。

```json
{
  "manifestEndpoints": [],
  "downloadBases": []
}
```

| 字段 | 填写内容 | 用途 |
| --- | --- | --- |
| `manifestEndpoints` | 自有桌面 `latest.json` 的完整 HTTPS URL，通常为 `<base>/latest/latest.json` | 新客户端优先读取自有元数据 |
| `downloadBases` | 自有下载根路径，通常以 `/releases` 结尾（无末尾 `/`） | 从 `<base>/v<版本>/<文件名>` 下载；Android 从 `<base>/latest/release.json` 检查更新，从 `<base>/v<版本>/release.json` 获取该版本资产 |

两者都只接受 HTTPS，不接受认证信息、查询参数或片段。文件名和版本分别进行 URL
路径编码，不要手工拼入未经编码的用户输入。允许多个自有源，按配置顺序使用。
这些是随构建嵌入的公开分发地址，不能放访问密钥；配置变更须重新构建客户端。
仅填写已由维护者控制且验证可用的实际地址，空数组可用于暂缓启用。

以下命令仅供接入时使用。在环境中先设置真实的 `SOLOSOUL_UPDATE_BASE`，再运行；
未设置时会报错，不会写入占位地址：

```bash
cd tauri
node --input-type=module <<'JS'
import fs from 'node:fs';
import { normalizeDownloadBase } from './scripts/generate-latest-json.js';
const base = normalizeDownloadBase(process.env.SOLOSOUL_UPDATE_BASE);
fs.writeFileSync('src-tauri/update-sources.json', JSON.stringify({
  manifestEndpoints: [`${base}/latest/latest.json`],
  downloadBases: [base],
}, null, 2) + '\n');
JS
```

生成器不会隐式采用配置文件中的地址。发版时必须显式传
`--download-base-url "$SOLOSOUL_UPDATE_BASE"`，使发布清单与已构建客户端的配置一致。

## 2. 存储路径与 HTTP 行为

同一版本的资产在 GitHub 和所有自有源上必须字节一致；上传已构建、已签名的原文件，
不要重新压缩 updater 归档或替换 APK。以 `${base}` 对应的存储前缀为根：

```text
releases/
├── v<版本>/
│   ├── SoloSoul_<版本>_arm64.app.tar.gz
│   ├── SoloSoul_<版本>_x64-setup.exe
│   ├── SoloSoul_<版本>_universal-release.apk
│   ├── SoloSoul_<版本>_universal-release.apk.sha256
│   ├── SoloSoul_<版本>_universal-release.apk.sha256.minisig
│   ├── latest.json
│   ├── latest-mirror-ghfast.json
│   ├── latest-mirror-ghproxy-net.json
│   ├── latest-mirror-ghproxy.json
│   ├── latest-mirror-ghps.json
│   └── release.json
└── latest/
    ├── latest.json
    └── release.json
```

`v<版本>/release.json` 和 `latest/release.json` 两个位置都要有文件。
根路径、对象 key 的大小写和文件名必须与清单精确一致。桌面 `.sig` 内容已嵌入清单，
独立 `.sig` 文件可归档但不要求供客户端下载。

存储/CDN 需要满足：

- 公共 HTTPS GET，证书链有效；无需登录、Cookie 或临时令牌，也不能返回挑战/验证码页面。
- 包和校验和支持单段 Range 请求，正确返回 **206 + Content-Range + Content-Length**。
  不可忽略 Range 而返回整个包，也不可把错误页包装为 200/206。
- 关闭安装包的透明压缩或内容变换，保持字节、长度和 Range 偏移不变。
- 版本目录按不可变资产维护，可长时间缓存；`latest/` 下元数据使用 `no-cache` 或较短缓存期，
  发布时主动刷新。JSON 建议使用 `application/json`，最大不超过 512 KiB。
- 保留旧版本及其签名/校验和，避免已获取旧清单的客户端下载时遇到 404。

签名仍由原私钥生成，客户端仍使用原信任公钥验证；CDN 只承担公开文件分发。
Range 首段检查只能验证传输协议、大小和首段内容，不能证明完整包的签名或完整性。

## 3. 生成候选清单并离线验证

先完成 [发布流程](release_process.md) 的构建、APK 校验和签名与统一签名。
已有签名不能因为更换下载地址而重建。先执行原签名自检：

```bash
bash scripts/verify-release-signatures.sh SoloSoul-Releases
```

在项目根目录执行以下命令；`SOLOSOUL_VERSION` 为本次发布版本，
`SOLOSOUL_UPDATE_BASE` 为真实自有分发根路径。输出到单独 staging 目录，便于先审阅候选
清单，不覆盖已有发布资产：

```bash
node tauri/scripts/generate-latest-json.js \
  "${SOLOSOUL_VERSION:?请指定本次版本}" SoloSoul-Releases \
  /tmp/solosoul-update-staging/latest.json \
  --notes-file "SoloSoul-Releases/release-notes-v${SOLOSOUL_VERSION}.md" \
  --download-base-url "${SOLOSOUL_UPDATE_BASE:?请配置真实分发根路径}" \
  --no-probe
node tauri/scripts/verify-update-distribution.js \
  /tmp/solosoul-update-staging/latest.json \
  --artifacts-dir SoloSoul-Releases --offline
```

始终生成四个固定 legacy 镜像清单。`--no-probe` 完全离线，不减少清单数量。
没有该参数时，生成器探测真实包地址并报告失败，但不会根据一次网络故障删除兼容文件；
在资产尚未上传时，404 是预期诊断，不能据此判断整个发布已验收通过。

有 APK 时生成器要求该 APK 的 `.sha256` 和 `.sha256.minisig` 都存在，避免发布
缺少完整性证明的 Android 元数据。`release.json` 与桌面清单使用相同版本、正文与时间。

## 4. 上传与切换顺序

1. 把版本资产和 staging 中的清单上传到所有自有源的 `v<版本>/`。先完成上传，再检查
   对象大小、metadata 和 Range 行为；此时尚不改变客户端正在读取的 `latest/` 文件。
2. 用下述只读在线命令验证版本目录元数据、所有桌面包、APK、校验和和 minisig 的真实 URL。
   任一错误都以非零退出。相同 CDN URL 会去重，不重复读取。
3. 通过后，将已验收的 `latest.json` 和 `release.json` 切换到 `latest/`，刷新对应缓存。
   两份文件不能跨请求原子切换时，应紧邻更新；每份清单内部引用的版本资产必须已经全部存在。
4. GitHub Release 同时保留原始包、APK 校验和/签名、主清单、**全部四份 legacy 清单**
   及 `release.json` 副本。GitHub 的 `releases/latest` 指针仍按原发布流程验证。
5. 再验收 `latest/` 两份元数据，分别在国内网络和海外网络进行实际客户端检查更新/安装验证。
   本地一次联网成功不能代表所有运营商或地区都可达。

版本目录上线、尚未切换 `latest/` 时：

```bash
node tauri/scripts/verify-update-distribution.js \
  /tmp/solosoul-update-staging/latest.json --artifacts-dir SoloSoul-Releases \
  --manifest-url "${SOLOSOUL_UPDATE_BASE}/v${SOLOSOUL_VERSION}/latest.json" \
  --release-url "${SOLOSOUL_UPDATE_BASE}/v${SOLOSOUL_VERSION}/release.json"
```

完成 `latest/` 切换后：

```bash
node tauri/scripts/verify-update-distribution.js \
  /tmp/solosoul-update-staging/latest.json --artifacts-dir SoloSoul-Releases \
  --manifest-url "${SOLOSOUL_UPDATE_BASE}/latest/latest.json" \
  --release-url "${SOLOSOUL_UPDATE_BASE}/latest/release.json" \
  --release-url "${SOLOSOUL_UPDATE_BASE}/v${SOLOSOUL_VERSION}/release.json"
```

`--manifest-url` 和 `--release-url` 可重复，以检查多个 metadata 入口。前者与本地
`latest.json` 比较，后者与本地 `release.json` 比较。若未启用 CDN，各 legacy 清单的包
URL 指向不同代理，验证器会检查所有这些 URL，任何不可用来源都明确报告失败。

验收脚本只读文件和网络，不上传、不改清单、不下载整个安装包。每个安装包最多读取
1024 字节；metadata 有 512 KiB 硬上限，连接和正文读取共享 8 秒超时。检查包括
兼容文件齐全、版本/正文/签名一致、Android 资产三件套、远端 metadata 精确一致、
Range/Content-Range/总大小及首段与本地包一致。完整 APK SHA-256、minisign、桌面 updater
验签仍遵循原流程和客户端验证，不能以本脚本通过替代。

## 5. 已安装旧版本的兼容边界

例如桌面 v2.12.1 的 metadata 入口已固化，不能通过服务端配置让它直接访问新自有
`latest.json`。把同一份保留签名、包 URL 已指向 CDN 的候选清单发布到原 Release 上的
`latest.json` 和全部 `latest-mirror-*.json`，旧客户端只要能取得任一兼容清单，就可以
从 CDN 下载新包，升级后才能使用编译进新版本的自有 metadata 入口。

如果旧客户端的 GitHub 和所有代理 metadata 入口都不可达，仍需从自有分发地址手动
安装一次新版本；仅迁移包地址无法修复完全不可达的旧 metadata 入口。

Android 的旧版检查入口同样固化。新 `release.json` 服务于支持自有源的新构建；
GitHub Release 中原 APK、`.sha256`、`.sha256.minisig` 仍需保留，供旧版路径回退。

## 6. 离线回归测试

```bash
node --test tauri/scripts/update-distribution.test.js
```

测试使用临时假产物和内存 HTTP 响应，不连接真实域名、不修改现有 Release 资产。
覆盖路径编码、恶意 URL、固定兼容清单、签名一致性、Android metadata、Range 响应、
超大正文、超时、重定向、远端清单一致性，以及生成/验收两个 CLI 的离线端到端行为。
