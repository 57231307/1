# 加密算法合规性扫描报告

- 扫描日期: 2026-08-22
- 扫描范围: `backend/src/`、`backend/Cargo.toml`
- 扫描任务: 4.9 加密算法合规性扫描（禁用弱算法）

## 1. 弱算法使用情况

### 1.1 弱哈希算法（MD5 / SHA1）

| 算法 | 文件 | 行号 | 用途 | 风险等级 |
|------|------|------|------|----------|
| SHA1 | `backend/src/services/email_service.rs` | 13, 17, 546, 750, 773, 803-804 | 阿里云 DirectMail RPC V1 API 签名（HMAC-SHA1） | 低（受控） |
| SHA1 | `backend/Cargo.toml` | 114 | 依赖声明 `sha1 = "0.10"` | - |

**MD5**: 全项目无使用。

**SHA1 使用说明**：唯一用途是阿里云 DirectMail `SingleSendMail` API 的 RPC V1 签名。该签名规范由阿里云官方强制规定（`SignatureMethod = HMAC-SHA1`），属于第三方 API 协议约束，非项目自身加密设计选择。SHA1 在此场景作为 HMAC 的底层哈希函数用于消息认证而非密码存储或抗碰撞场景，且密钥为阿里云 AccessKeySecret，未用于保护用户敏感数据。判定为**受控使用**，但建议补充注释说明不可替换的原因，避免后续误用为通用哈希。

### 1.2 弱加密模式（DES / RC4 / AES-ECB）

```
grep -rnE "\b(des|Des|DES|rc4|RC4|aes_ecb|ECB)\b" backend/src/ backend/Cargo.toml
（过滤 Deserialize/description 等误报后无匹配）
```

**结论**: 全项目未使用 DES、RC4、AES-ECB 等弱加密模式或弱分组密码。

## 2. 安全算法清单

### 2.1 Cargo.toml 依赖（`backend/Cargo.toml`）

| 算法库 | 版本 | 行号 | 用途 |
|--------|------|------|------|
| `argon2` | 0.5 | 31 | 密码哈希（Argon2id） |
| `sha2` | 0.10 | 115 | SHA-256/512 通用哈希 |
| `hmac` | 0.12 | 113 | HMAC 消息认证码 |
| `sha1` | 0.10 | 114 | 阿里云 API 签名（受控） |
| `fastrand` | 2.4.0 | 111 | 非密码学随机数（业务编号） |

未引入 `md-5`、`des`、`rc4`、`aes`（ECB 模式）等弱算法 crate。

### 2.2 密码哈希（Argon2）

项目密码哈希统一使用 **Argon2id**，参数符合安全基线：

| 文件 | 行号 | 参数 | 用途 |
|------|------|------|------|
| `backend/src/services/auth_service_ops/auth.rs` | 133-161 | Argon2id V0x13, m=65536, t=3, p=4 | 用户密码验证 + 哈希 |
| `backend/src/cli/admin.rs` | 67-76 | Argon2id V0x13, m=65536, t=3, p=4 | 管理员 CLI 创建用户 |
| `backend/src/bin/hash_password.rs` | 17-27 | Argon2id V0x13, m=65536, t=3, p=4 | 密码哈希工具 |
| `backend/src/services/totp_service.rs` | 113-197 | Argon2id（同参数） | TOTP 恢复码哈希存储 |

**参数合规性**：内存 64MiB、迭代 3 次、并行 4 线程，Salt 使用 `OsRng`（密码学安全随机源），符合 OWASP Argon2id 推荐基线。恢复码与用户密码采用同一算法，设计一致。

## 3. 随机数生成安全性

### 3.1 非密码学随机（`fastrand`）

`fastrand` 在项目中用于生成业务编号后缀，共 5 处直接调用 + 24 处经 `utils/random.rs` 间接调用：

| 文件 | 行号 | 用途 | 安全影响 |
|------|------|------|----------|
| `backend/src/utils/random.rs` | 7-21 | `random_4_digit` / `random_6_digit` / `random_alphanumeric` 工具函数 | 业务编号后缀 |
| `backend/src/services/email_service.rs` | 761 | 阿里云 API `SignatureNonce` | 防重放（低风险） |
| `backend/src/observability/trace_context.rs` | 120-121 | `span_id`（64bit）生成 | 链路追踪 ID |
| `backend/src/middleware/rate_limit.rs` | 56 | 1/1000 采样 | 限流采样 |
| 24 处业务服务 | - | 单据编号后缀（订单/批次/凭证等） | 仅保证唯一性 |

`fastrand` 是非密码学安全 PRNG，用于业务编号生成场景**可接受**（目的是避免编号冲突，非安全令牌）。

### 3.2 密码学安全随机（`OsRng`）

涉及安全敏感场景均使用 `argon2::password_hash::rand_core::OsRng`（密码学安全随机源）：

| 文件 | 行号 | 用途 |
|------|------|------|
| `backend/src/cli/admin.rs` | 69 | Argon2 Salt 生成 |
| `backend/src/services/auth_service_ops/auth.rs` | 154-161 | 密码哈希 Salt |
| `backend/src/services/totp_service.rs` | 115 | TOTP 恢复码 Salt + 明文生成 |
| `backend/src/bin/hash_password.rs` | - | 密码哈希工具 Salt |

### 3.3 需关注项：API Key 生成

`backend/src/services/api_key_service.rs:41` 使用 `random_alphanumeric(32)`（基于 `fastrand`）生成 API Key：

```rust
let key = random::random_alphanumeric(32);
```

API Key 属于**安全敏感凭据**，使用非密码学安全的 `fastrand` 生成存在被预测的理论风险。建议改用 `OsRng` 或 `rand::rngs::OsRng` 生成，这是本次扫描发现的**唯一需要修复的随机数问题**。

### 3.4 `rand::thread_rng` / `rand::random`

全项目未使用 `rand::thread_rng` 或 `rand::random`。

## 4. 合规结论

| 检查项 | 结果 | 弱算法使用数 |
|--------|------|--------------|
| MD5 使用 | 未使用 | 0 |
| SHA1 使用 | 1 处（阿里云 API 签名，受控） | 1（受控） |
| DES / RC4 / AES-ECB | 未使用 | 0 |
| 密码哈希算法 | Argon2id（合规） | 0 |
| 随机数（安全场景） | OsRng（合规），1 处 API Key 误用 fastrand | 1（待修复） |

**弱算法使用总数: 1**

- 受控弱算法: 1（SHA1，阿里云 API 协议约束，无法替换）
- 待修复问题: 1（API Key 生成误用 `fastrand`，应改用 `OsRng`）

### 整体评价

项目加密算法选型**基本合规**：密码哈希统一采用 Argon2id 且参数达标，安全随机数使用 OsRng，未引入 MD5/DES/RC4/AES-ECB 等已知弱算法。唯一的 SHA1 使用受第三方 API 协议约束，属合理例外。需修复项为 `api_key_service.rs` 中 API Key 生成改用密码学安全随机源。

### 建议修复项

1. `backend/src/services/api_key_service.rs:41` — 将 API Key 生成从 `fastrand` 改为 `OsRng`，避免凭据可预测风险
2. `backend/src/services/email_service.rs:546` — 补充注释说明 SHA1 为阿里云协议强制要求，禁止扩散到其他场景
