# 代码质量加固需求文档

## 引言

本文档覆盖 Bingxi Management Platform 的 5 项代码质量加固任务：CI 测试门禁修复、预留功能接入路由、TODO/FIXME 标记管理、前端类型安全加固、安全审计缺口补齐。

## 术语表

- **ci-test-rust**: CI 流水线中运行 Rust 测试并收集覆盖率的 job
- **EXIT_CODE**: 进程退出码，0 表示成功，非 0 表示失败
- **假绿**: 测试编译失败但 CI job 报告为 success 的现象
- **struct never constructed**: Clippy 警告，表示 struct 已定义但从未被实例化
- **EARS**: Easy Approach to Requirements Syntax，需求表述规范

---

## 需求 1：CI 测试编译门禁修复

**用户故事**: 作为项目维护者，我希望 ci-test-rust 在测试编译失败时报告 failure，以避免假绿导致破损代码进入 main 分支。

### 验收标准

1. WHEN `cargo llvm-cov nextest` 返回非零 EXIT_CODE 且 FAILED 计数为 0 时，ci-test-rust SHALL 报告 failure（而非 success）
2. WHEN 测试编译失败（EXIT_CODE 非零、FAIL 行为 0）时，ci-test-rust SHALL 在报告中明确标注"编译失败"并输出编译错误日志
3. WHILE EXIT_CODE 为 0 且 FAILED 为 0 时，ci-test-rust SHALL 报告 success
4. IF EXIT_CODE 非零且 FAILED 大于 0，ci-test-rust SHALL 保持现有行为报告 failure

---

## 需求 2：预留功能接入路由

**用户故事**: 作为开发者，我希望已定义的 service struct 被接入路由或明确标记为预留，以消除 Clippy dead_code 警告并保持代码意图清晰。

### 验收标准

1. WHEN service struct 已定义且对应业务功能已实现时，系统 SHALL 将该 service 接入路由并在 routes/ 中注册对应端点
2. WHEN service struct 已定义但业务功能尚未实现时，系统 SHALL 在 struct 上添加 `#[allow(dead_code)]` 并附带注释说明预留原因和计划
3. WHILE 接入路由后，系统 SHALL 确保对应 handler 调用真实 service 逻辑（而非返回硬编码空数据或"功能暂未实现"）
4. IF service 已接入路由但缺少测试，系统 SHALL 补充基本的集成测试覆盖

---

## 需求 3：TODO/FIXME 标记管理

**用户故事**: 作为项目维护者，我希望所有 TODO/FIXME 标记被记录到技术债务清单，以便跟踪和逐步偿还。

### 验收标准

1. WHEN 代码中存在 TODO/FIXME/HACK/XXX 标记时，系统 SHALL 在 `.monkeycode/specs/code-quality-hardening/debt-tracking.md` 中记录每个标记的位置、内容和优先级
2. WHEN 标记关联的修复已通过其他任务完成时，系统 SHALL 从代码中移除该标记
3. WHILE 新增代码中，开发者 SHALL 遵循项目规范：禁止新增无 issue 关联的 TODO/FIXME
4. IF 标记内容已过时或不再适用，系统 SHALL 直接移除该标记

---

## 需求 4：前端类型安全加固

**用户故事**: 作为前端开发者，我希望消除 `any` 类型滥用，以获得更好的类型检查和 IDE 提示。

### 验收标准

1. WHEN 前端代码中使用 `any` 类型且可替换为具体接口时，系统 SHALL 定义对应的 TypeScript 接口并替换 `any`
2. WHEN `any` 用于 API 响应处理时，系统 SHALL 替换为从后端 OpenAPI schema 自动生成的类型定义（或手写的等价接口）
3. WHILE 替换过程中，系统 SHALL 确保不引入新的类型错误（通过 `npm run type-check` 验证）
4. IF `any` 的使用是合理的（如泛型约束、第三方库兼容），系统 SHALL 保留 `any` 并添加 `// eslint-disable-next-line @typescript-eslint/no-explicit-any` 注释说明原因

---

## 需求 5：安全审计缺口补齐

**用户故事**: 作为安全负责人，我希望完成依赖漏洞扫描、供应链安全审计和加密算法合规检查，以满足生产环境安全要求。

### 验收标准

1. WHEN CI 流水线运行时，系统 SHALL 执行依赖漏洞扫描（cargo audit）并报告已知 CVE
2. WHEN 发现高危或严重漏洞时，系统 SHALL 阻塞 CI（fail_ci_if_error: true）
3. WHILE 审查第三方 crate 时，系统 SHALL 记录每个 crate 的用途、许可证和维护状态到 `.monkeycode/specs/code-quality-hardening/supply-chain-audit.md`
4. IF 项目使用了弱加密算法（如 MD5、SHA1 用于安全场景），系统 SHALL 替换为合规算法（SHA256+、AES-256 等）
5. WHEN 日志输出敏感信息时，系统 SHALL 确保 PII 数据已脱敏（通过 pii_mask 模块）
