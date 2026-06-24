# 安全漏洞详细记录

> 本文档记录 2026-06-24 周期性安全审计中确认的漏洞。
> 审计范围：仓库全量代码，重点关注高风险攻击面。
> 审计依据：可论证的端到端利用路径（仅记录已确认漏洞，不含推测性风险）。
>
> **维护规则**：修复完成的漏洞需立即从本文档移除（见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则）。

---

## 漏洞总览

| 编号 | 严重度 | 漏洞名称 | 状态 |
|------|--------|----------|------|
| #7 | 🟡 低危 | 弱密码黑名单策略不严 | 未修复 |

> **已修复**（详见 PR #250、PR #251、PR #252、PR #253、PR #254）：
> - ✅ #1 静态资源路径遍历漏洞（PR #250）
> - ✅ #2 WebSocket 通知服务认证绕过（PR #250）
> - ✅ #3 系统初始化接口匿名访问风险（PR #251）
> - ✅ #4 错误信息泄露内部细节（PR #252）
> - ✅ #5 API Key 撤销后仍可被冒用（PR #253）
> - ✅ #6 内存速率限制器多实例失效（PR #254）
> - ✅ #8 调试模式错误响应泄露堆栈信息（PR #252）

---


## 漏洞 #7：弱密码黑名单策略不严

### 基础信息

- **严重度**：🟡 低危（Low）
- **CWE 分类**：CWE-521 弱密码要求
- **CVSS 3.1 评分估计**：3.7（AV:N/AC:H/PR:N/UI:N/S:U/C:L/I:N/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[utils/password_validator.rs](file:///workspace/backend/src/utils/password_validator.rs)
- 关键代码行：[84-100](file:///workspace/backend/src/utils/password_validator.rs#L84-L100)、[152-159](file:///workspace/backend/src/utils/password_validator.rs#L152-L159)

### 攻击者画像

- **类型**：已认证用户（注册/改密流程）
- **所需权限**：需要基本用户权限

### 可控输入向量

密码字段。

### 完整利用路径

1. `validate_password_strength` 通过 `validator` crate 的 `custom` 函数校验
2. 密码策略的 `is_valid = errors.is_empty()` 规则下，**所有校验项失败会平权扣分**
3. COMMON_PASSWORDS 检查仅扣 30 分（[第 158 行](file:///workspace/backend/src/utils/password_validator.rs#L158)）
4. 一个含 16+ 字符、含大小写数字特殊字符的密码即使命中 COMMON_PASSWORDS 黑名单，仍可能因分数 ≥ 50 通过 Medium 等级校验

### 影响分析

- **弱密码（虽然长了但被列入常见黑名单）可能被接受**
- **缺少强密码检测**：如键盘连续字符、特定人名、生日等
- **黑名单覆盖范围有限**：仅 15 个常见密码

### 修复建议

#### 必做修复

1. COMMON_PASSWORDS 命中应作为硬性拒绝（`is_valid = false`），而非仅扣分
2. 考虑接入 HIBP（Have I Been Pwned）API 或更大规模的弱密码库

#### 示例代码

```rust
// 修改 common passwords 检查逻辑
let lower_password = password.to_lowercase();
if COMMON_PASSWORDS
    .iter()
    .any(|common| lower_password.contains(common))
{
    errors.push("密码过于常见，不安全".to_string());
    // 不再扣分，直接保留在 errors 中，由 is_valid = errors.is_empty() 决定
}

// 或者更严格：作为硬性拒绝
if COMMON_PASSWORDS.iter().any(|common| lower_password == *common) {
    return PasswordValidationResult {
        strength: PasswordStrength::VeryWeak,
        is_valid: false,
        errors: vec!["密码过于常见，不允许使用".to_string()],
    };
}
```

---

## 修复优先级建议

| 优先级 | 漏洞编号 | 预计修复时间 | 风险等级 |
|--------|----------|--------------|----------|
| P0 - 立即修复 | #1, #2 | 1-2 天 | 高危 |
| P1 - 本周内修复 | #3 | 3-5 天 | 中危 |
| P2 - 计划修复 | #4, #5, #6, #7, #8 | 2-3 周 | 低危 |

---

## 审计方法论

### 审计范围

1. **认证与访问控制**：登录流程、会话管理、角色/权限校验
2. **注入向量**：原始 SQL 查询、Shell 命令拼接、模板渲染、文件路径操作
3. **外部交互**：Webhook 处理器、出站网络请求、第三方 API 集成
4. **敏感数据处理**：代码或配置中的密钥、凭证或 PII 的日志记录、加密实践

### 审计方法

1. 梳理代码库架构 - 理解入口点、信任边界、数据流转
2. 系统性检查高风险攻击面
3. 对每个潜在发现，从攻击者可控输入到影响结果追踪完整代码路径
4. 仅保留能具体证明可利用性的发现

### 证据要求

每个报告的问题必须清晰说明：
- 攻击者画像（外部用户、已认证用户、内部服务等）
- 其可控的输入向量
- 从输入到漏洞的确切代码路径
- 造成的影响（数据泄露、权限提升、拒绝服务等）
- 建议的修复方案

---

**记录时间**：2026-06-24
**审计工具**：人工审计
**下次审计**：建议 30 天后
