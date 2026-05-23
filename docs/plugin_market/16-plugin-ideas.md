## 16. 插件创意库（Plugin Ideas）

> 本文档收集适合 SoloSoul 生态的插件创意，按网络依赖度分类。
> SoloSoul 核心哲学是「本地优先、隐私优先」，因此**优先推荐零网络依赖的纯本地插件**。

---

## 目录

- [16.1 纯本地插件（零网络）](#161-纯本地插件零网络)
  - [第一层：高价值 + 开发简单](#第一层高价值--开发简单)
  - [第二层：需要一定计算量](#第二层需要一定计算量)
  - [第三层：有创意、展示性强](#第三层有创意展示性强)
- [16.2 需要联网的插件](#162-需要联网的插件)
- [16.3 技术可行性速查表](#163-技术可行性速查表)
- [16.4 推荐开发排序](#164-推荐开发排序)

---

## 16.1 纯本地插件（零网络）

这些插件仅依赖 `request_field`（读取 Vault 加密字段）、`get_timestamp`（获取时间戳）和 `log`（日志记录），不需要 `post_json`。

### 第一层：高价值 + 开发简单

| 插件 ID | 名称 | 功能描述 | 读取字段 | 输出 |
|---------|------|----------|----------|------|
| `com.solosoul.official.mrz-encoder` | **MRZ 编码器** | 将 Vault 中的护照/身份证信息编码为 ICAO Doc 9303 标准机读区格式 | `passport.*`, `idCard.*` | MRZ 字符串（两行或三行） |
| `com.solosoul.official.id-validator` | **证件号码校验器** | 校验各国证件号码的校验位：中国身份证 18 位模 11 校验、美国 SSN 格式、英国 NI Number 等 | `idCard.number`, `passport.number`, `taxId.*` | 校验结果（valid/invalid + 错误位置） |
| `com.solosoul.official.totp-gen` | **TOTP 生成器** | 基于 Vault 中存储的 2FA Secret，按 RFC 6238 生成 6 位动态验证码 | `security.totp_secret`（需新增字段） | 6 位验证码 + 剩余有效秒数 |
| `com.solosoul.official.address-fmt` | **地址格式化器** | 将 Vault 中的地址按目标国家/地区格式规范化（中/英/日/德等） | `address.*` | 格式化后的地址字符串 |
| `com.solosoul.official.emergency-card` | **紧急联系卡** | 生成紧急情况下使用的医疗/联系信息卡片 | `identity.*`, `contact.*`, `medical.*` | 结构化紧急信息文本 |

### 第二层：需要一定计算量，但纯本地

| 插件 ID | 名称 | 功能描述 | 读取字段 | 输出 |
|---------|------|----------|----------|------|
| `com.solosoul.official.expiry-guardian` | **证件到期卫士** | 扫描所有证件（护照、签证、身份证、信用卡）有效期，按紧急程度排序 | `passport.expiryDate`, `visa.expiryDate`, `card.expiryYear` | 即将过期清单（30/60/90/180 天分级） |
| `com.solosoul.official.travel-footprint` | **旅行足迹分析** | 分析 Vault 中的签证与旅行记录，生成到访国家统计、最常去地区、旅行时间线 | `visa.*`, `travel.*`, `passport.*` | JSON 统计报告 |
| `com.solosoul.official.resume-builder` | **简历生成器** | 从 Vault 提取教育、职业、技能、语言信息，生成标准简历 | `education.*`, `employment.*`, `skill.*`, `language.*` | Markdown / JSON 简历 |
| `com.solosoul.official.form-prefiller` | **表单预填助手** | 根据目标场景（签证申请、银行开户、酒店入住）生成字段映射表 | `identity.*`, `contact.*`, `address.*`, `employment.*` | 字段映射 JSON（目标表单字段 → Vault 值） |
| `com.solosoul.official.password-health` | **密码健康度检查** | 分析 Vault 中各账户密码的长度和复杂度分布（仅返回评分，不泄露密码） | `security.password_strength`（需新增字段） | 健康度评分报告 |
| `com.solosoul.official.tax-profile` | **税务档案摘要** | 根据居住国、收入来源国、税务居民身份，汇总税务申报基础数据 | `taxId.*`, `address.*`, `employment.*` | 税务数据摘要 JSON |

### 第三层：有创意、展示性强

| 插件 ID | 名称 | 功能描述 | 读取字段 | 输出 |
|---------|------|----------|----------|------|
| `com.solosoul.official.digital-will` | **数字遗产指示** | 基于 Vault 数据生成数字遗产分配建议（紧急情况下的账户/资产处理方案） | 用户授权的全字段 | 结构化遗产指示文本 |
| `com.solosoul.official.identity-timeline` | **身份时间线** | 按时间线展示用户身份变迁：学历 → 工作 → 签证 → 资产获取 | `education.*`, `employment.*`, `visa.*`, `property.*` | 时间线 JSON（含日期节点） |
| `com.solosoul.official.namecard-gen` | **数字名片生成器** | 生成加密数字名片（二维码包含加密联系信息，需 SoloSoul 扫码解密） | `contact.*`, `identity.*` | vCard 数据 / 二维码字节流 |
| `com.solosoul.official.doc-checklist` | **材料清单检查器** | 根据目标场景（如"申请日本签证"）反推 Vault 中已有/缺失的材料 | `passport.*`, `financial.*`, `employment.*`, `travel.*` | 已有 ✓ / 缺失 ✗ 清单 |
| `com.solosoul.official.data-completeness` | **档案完整度扫描** | 扫描 Vault 所有 section，计算完整度百分比，提示缺失的关键字段 | 全字段（分 section 扫描） | 完整度报告（如"身份区 100%，财务区 40%"） |
| `com.solosoul.official.unit-converter` | **智能单位转换** | 基于 Vault 中设置的偏好单位（货币、度量衡）进行上下文感知的单位转换 | `preferences.*`（需新增） | 转换结果 |

---

## 16.2 需要联网的插件

> 这些插件需要 `post_json` Host Function，适合第二阶段开发。

| 插件 ID | 名称 | 功能描述 | 读取字段 | 网络行为 |
|---------|------|----------|----------|----------|
| `com.solosoul.official.slotgo` | **SlotGo** | UK Visa 预约系统（已开发框架） | `passport.*` | POST 预约请求到 TLScontact API |
| `com.solosoul.official.visa-appointment` | **申根预约助手** | 申根签证 slot 监控与预约 | `passport.*`, `travel.*` | 轮询各国领事馆预约系统 |
| `com.solosoul.official.bank-kyc` | **银行 KYC 助手** | 自动填写银行开户 KYC 表单 | `identity.*`, `address.*`, `financial.*` | POST 到银行开户 API |
| `com.solosoul.official.tax-filing` | **税务申报助手** | 根据 Vault 数据自动填写税务申报表 | `taxId.*`, `financial.*`, `employment.*` | POST 到税务局电子申报系统 |
| `com.solosoul.official.llm-reason-gen` | **申请理由生成器** | 使用 LLM 生成签证/学校申请的个人陈述（脱敏后发送） | `identity.*`, `education.*`, `employment.*` | POST 到 LLM API（数据脱敏） |

---

## 16.3 技术可行性速查表

以当前 SDK Host Functions 评估：

| 插件 | 所需 Host Functions | 计算复杂度 | 开发难度 | 备注 |
|------|---------------------|-----------|---------|------|
| MRZ Encoder | `request_field`, `log` | 低 | ⭐ | 纯字符串拼接，ICAO 9303 规范公开 |
| ID Validator | `request_field`, `log` | 低 | ⭐ | 各国校验位算法均为公开数学运算 |
| TOTP Generator | `request_field`, `get_timestamp`, `log` | 中 | ⭐⭐ | 标准 RFC 6238，需 HMAC-SHA1 |
| Expiry Guardian | `request_field`, `get_timestamp`, `log` | 低 | ⭐ | 日期解析 + 比较 |
| Address Formatter | `request_field`, `log` | 低 | ⭐ | 模板替换 |
| Emergency Card | `request_field`, `log` | 低 | ⭐ | 字段拼接 |
| Resume Builder | `request_field`, `log` | 中 | ⭐⭐ | 需设计多段模板（教育→工作→技能） |
| Form Prefiller | `request_field`, `log` | 中 | ⭐⭐ | 需维护常见表单字段到 Vault 字段的映射表 |
| Travel Footprint | `request_field`, `log` | 中 | ⭐⭐ | 需国家代码 → 地区映射表 |
| Password Health | `request_field`, `log` | 中 | ⭐⭐ | 需 Vault 中密码以可评分格式存储 |
| Tax Profile | `request_field`, `log` | 高 | ⭐⭐⭐ | 需小型各国税法规则表 |
| Digital Will | `request_field`, `log` | 低 | ⭐⭐ | 需法律免责声明模板 |
| Identity Timeline | `request_field`, `log` | 中 | ⭐⭐ | 需日期排序 + 时间线格式化 |
| Namecard Gen | `request_field`, `log` | 中 | ⭐⭐ | 需 vCard 格式知识 |
| Doc Checklist | `request_field`, `log` | 中 | ⭐⭐ | 需场景→所需字段映射表 |
| Data Completeness | `request_field`, `log` | 低 | ⭐⭐ | 需遍历所有 section 定义 |

---

## 16.4 推荐开发排序

基于「用户价值 × 开发速度 × 展示效果」综合评估：

### Phase A（立即开发）

1. **Expiry Guardian** — 每个用户都有证件，到期提醒是高频刚需，纯本地计算，代码量小，展示效果直观。
2. **ID Validator** — 开发极快（几十行 WASM），能立刻提升数据录入质量，用户体验直接。
3. **MRZ Encoder** — 与已有的 OCR MRZ 解析形成技术闭环（读→存→编码），有成就感。

### Phase B（短期跟进）

4. **Address Formatter** — 实用性强，出国填表高频场景。
5. **Emergency Card** — 安全相关，符合 SoloSoul 隐私品牌调性。
6. **TOTP Generator** — 安全工具类，可替代独立 Authenticator App。

### Phase C（中期填充）

7. **Resume Builder** — 展示 SoloSoul 作为「数字孪生」的价值。
8. **Travel Footprint** — 可视化效果好，适合 Demo 展示。
9. **Form Prefiller** — 需要维护较大量的映射表，但用户价值极高。

### Phase D（长期探索）

10. **Digital Will / Identity Timeline / Data Completeness** — 偏概念展示，适合品牌传播和高级用户。

---

*本文档应保持开放，任何新插件创意可直接追加到对应分层中。*
