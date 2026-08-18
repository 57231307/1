# 代码质量加固技术设计

Feature Name: code-quality-hardening
Updated: 2026-08-15

## 描述

本设计覆盖 Bingxi Management Platform 的 5 项代码质量加固任务的实施方案。

---

## 1. CI 测试编译门禁修复

### 问题分析

`ci-test-rust` job 第 1298 行仅检查 `FAILED > 0`，未检查 `EXIT_CODE`。当测试编译失败时，nextest 不输出 `FAIL` 行，`FAILED=0`，但 `EXIT_CODE` 非零。当前逻辑将此场景误判为 success。

### 修复方案

在 `.github/workflows/ci-cd.yml` 第 1297-1303 行增加 EXIT_CODE 检查：

```bash
# 现有逻辑（第 1297-1303 行）
if [ "$FAILED" -gt "0" ]; then
    echo "❌ 发现 $FAILED 个测试失败，CI 失败（零容忍）"
    exit 1
fi

# 新增：检查 EXIT_CODE（覆盖编译失败、panic、信号终止等场景）
if [ "$EXIT_CODE" -ne "0" ]; then
    echo "❌ 测试进程异常退出（EXIT_CODE=$EXIT_CODE），CI 失败"
    echo "可能原因：编译失败、panic、OOM、信号终止"
    echo "详见 reports/llvm-cov-nextest-output.txt"
    exit 1
fi
```

同时更新报告模板，在"编译失败"场景输出编译错误摘要。

### 影响范围

- 文件：`.github/workflows/ci-cd.yml`
- 风险：低（仅增加检查，不改变现有逻辑）

---

## 2. 预留功能接入路由

### 现状分析

124 个 `struct never constructed` 分布在：
- occupational_health_service（10 个）
- ai_model_management_service（10 个）
- customer_team_share_service（7 个）
- social_insurance_service（6 个）
- pollution_monitoring_service（6 个）
- 其他模块（85 个）

### 分类策略

将 124 个 struct 分为 3 类：

| 类别 | 处理方式 | 预估数量 |
|------|----------|----------|
| A. 已有 handler 但未接入路由 | 接入 routes/ 注册端点 | ~30 |
| B. 已有 service 但无 handler | 编写 handler + 接入路由 | ~40 |
| C. 纯预留（无业务逻辑） | 添加 `#[allow(dead_code)]` + 注释 | ~54 |

### 实施步骤

1. **扫描分类**：遍历 124 个 struct，检查对应 handler/routes 是否存在
2. **A 类处理**：在 `routes/` 对应模块中添加 `.route()` 注册
3. **B 类处理**：编写 handler 调用 service，注册路由
4. **C 类处理**：添加 `#[allow(dead_code, reason = "预留功能：{描述}")]`
5. **验证**：cargo clippy 确认 dead_code 警告消除

### 约束

- 禁止返回硬编码空数据或"功能暂未实现"（需求 2.3）
- 接入路由后必须有基本的集成测试覆盖（需求 2.4）

---

## 3. TODO/FIXME 标记管理

### 现状

16 处 TODO/FIXME/HACK/XXX 标记分布在 backend/src/ 中。

### 实施方案

1. **提取清单**：`grep -rn "TODO\|FIXME\|HACK\|XXX" backend/src/` 生成清单
2. **分类处理**：
   - 已完成的修复 → 直接移除标记
   - 仍需跟进的 → 记录到 `debt-tracking.md`，关联 issue
   - 内容过时的 → 直接移除
3. **输出文件**：`.monkeycode/specs/code-quality-hardening/debt-tracking.md`

### debt-tracking.md 格式

```markdown
| 编号 | 文件:行号 | 标记类型 | 内容 | 优先级 | 关联 Issue |
|------|-----------|----------|------|--------|-----------|
| 1 | services/xxx.rs:42 | TODO | 描述 | P2 | #xxx |
```

---

## 4. 前端类型安全加固

### 现状

101 个文件使用 `any` 类型。

### 分批策略

按模块分批处理，每批 15-20 个文件：

| 批次 | 模块 | 文件数 |
|------|------|--------|
| 1 | api/ | ~20 |
| 2 | views/ (前半) | ~20 |
| 3 | views/ (后半) | ~20 |
| 4 | components/ + store/ | ~20 |
| 5 | utils/ + 其他 | ~21 |

### 替换规则

1. **API 响应**：定义 `interface XxxResponse { ... }`，替换 `any`
2. **事件处理**：使用 Vue/Element Plus 泛型类型
3. **第三方库**：查阅类型定义，使用正确泛型
4. **确实无法确定**：保留 `any` + eslint-disable 注释说明原因

### 验证

每批完成后执行 `npm run type-check`，确保无新增类型错误。

---

## 5. 安全审计缺口补齐

### 5.1 依赖漏洞扫描（cargo audit）

在 CI 中新增 `ci-audit-rust` job：

```yaml
ci-audit-rust:
  name: 🛡️ Rust 依赖审计
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5
    - name: 安装 cargo-audit
      run: cargo install cargo-audit
    - name: 执行漏洞扫描
      run: cargo audit
```

### 5.2 供应链安全审计

输出文件：`.monkeycode/specs/code-quality-hardening/supply-chain-audit.md`

内容：
- 每个第三方 crate 的用途、许可证、维护状态
- 高风险 crate（维护停滞、已知安全问题）清单
- 替代方案建议

### 5.3 加密算法合规

扫描代码中的加密使用：
- `grep -rn "md5\|sha1\|des\|rc4" backend/src/` 查找弱算法
- 替换为 SHA256+、AES-256 等合规算法

### 5.4 PII 脱敏验证

确认 `utils/pii_mask.rs` 覆盖所有敏感字段：
- 手机号、身份证号、银行卡号
- 邮箱地址
- 密码、密钥

---

## 实施顺序

| 阶段 | 任务 | 依赖 | 预估工作量 |
|------|------|------|-----------|
| 1 | CI 假绿修复 | 无 | 小（5 行改动） |
| 2 | TODO/FIXME 标记管理 | 无 | 小（16 处） |
| 3 | 预留功能接入路由 | 无 | 大（124 个 struct） |
| 4 | 前端类型安全加固 | 无 | 大（101 个文件） |
| 5 | 安全审计补齐 | 阶段 1 完成后 | 中（CI 配置 + 扫描） |

## References

- `.github/workflows/ci-cd.yml:1185-1305` - ci-test-rust job 定义
- `.monkeycode/docs/TECHNICAL_DEBT_REPORT.md` - 技术债务报告
- `.monkeycode/docs/PROJECT_HEALTH_REPORT.md` - 项目健康度报告
