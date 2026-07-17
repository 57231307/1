# 项目规则记忆（索引版）

> 本文件是项目的规则记忆，记录必须遵守的规则、指令、偏好和工作流规范。
> 第 8 次迭代（2026-07-17，MEMORY-SU.md 梳理合并排序：11 章压缩为 6 章 + project_rules.md 删除后改为内联说明 + 6.4 迭代记录精简为最近 5 条）

---

## 一、关键项目规则（必读，按优先级排序）

| 规则 | 分类 | 一句话核心 | 详细说明 |
|------|------|-----------|----------|
| 🔴 **规则 00** | PR | 每次修改后、推送 CI/CD 前必须评估关联影响，所有修改为代码级修改 | [MEMORY-SU §规则 00](file:///workspace/.monkeycode/MEMORY-SU.md#规则-00修改前关联影响评估强制pr2026-07-11-追加) |
| 🔴 **规则 0** | PR | 所有预留 API/功能/占位符/路由必须真实实现，禁止 stub/placeholder | [MEMORY-SU §规则 0](file:///workspace/.monkeycode/MEMORY-SU.md#规则-0真实实现强制pr2026-07-04-追加2026-07-17-合并规则-8) |
| 🔴 **规则 1** | PR | 功能扩展空间/预留扩展点/未来支持视为未实现，必须本批次实现 | [MEMORY-SU §规则 1](file:///workspace/.monkeycode/MEMORY-SU.md#规则-1扩展空间视为未实现pr2026-07-06-追加) |
| 🔴 **规则 2** | PR | 完全实现 = 完整实现，100% 跟规划一致，禁止部分实现 | [MEMORY-SU §规则 2](file:///workspace/.monkeycode/MEMORY-SU.md#规则-2完全实现等于完整实现pr2026-07-06-追加) |
| 🔴 **规则 3** | IR | 成品导入/导出使用 .xlsx/.docx，禁止 CSV/txt/rtf/html 作为成品 | [MEMORY-SU §规则 3](file:///workspace/.monkeycode/MEMORY-SU.md#规则-3项目成品导入导出文档格式ir-个人规则2026-07-06-追加) |
| 🔴 **规则 4** | IR | `///` 注释精简为 1 行（首选），最多 2 行，禁止 3 行+注释块 | [MEMORY-SU §规则 4](file:///workspace/.monkeycode/MEMORY-SU.md#规则-4项目文件--注释精简规则ir-个人规则2026-07-06-追加) |
| 🔴 **规则 5** | PR | 每 30 批次 E2E 测试（独立工作流不阻塞主 CI），按 20/28/29 节奏监控 | [MEMORY-SU §规则 5](file:///workspace/.monkeycode/MEMORY-SU.md#规则-5e2e-测试加强pr2026-07-08-追加2026-07-10-批次-262-修订) |
| 🔴 **规则 6** | PR | 测试 mock 数据禁止硬编码，必须抽取到 fixtures 文件 | [MEMORY-SU §规则 6](file:///workspace/.monkeycode/MEMORY-SU.md#规则-6测试-mock-数据禁止硬编码pr2026-07-08-追加) |
| 🔴 **规则 7** | IR | 禁止简洁方案，采用最合理/最准确/最符合业务需求的方案 | [MEMORY-SU §规则 7](file:///workspace/.monkeycode/MEMORY-SU.md#规则-7禁止简洁方案ir-个人规则2026-07-08-追加) |
| 🔴 **规则 8** | PR | 已合并到规则 0（2026-07-17），保留编号避免破坏引用 | [MEMORY-SU §规则 8](file:///workspace/.monkeycode/MEMORY-SU.md#规则-8真实实现强制已合并到规则-02026-07-17) |
| 🔴 **规则 9** | IR | 个人规则（IR）高于项目规则（PR）；优先级 IR > PR > PH > IH | [MEMORY-SU §规则 9](file:///workspace/.monkeycode/MEMORY-SU.md#规则-9个人规则高于项目规则ir-个人规则2026-07-08-追加2026-07-17-与规则-18-对齐) |
| 🔴 **规则 10** | PR | 每 15 批整理归档 + 实时归档；MEMORY.md 只存规则，doto.md 只存未完成任务 | [MEMORY-SU §规则 10](file:///workspace/.monkeycode/MEMORY-SU.md#规则-10记忆文件定期整理归档--实时归档pr2026-07-08-追加2026-07-10-修正2026-07-14-二次修正) |
| 🔴 **规则 11** | PR | 符合中国法律法规（个人信息保护法/数据安全法/网络安全法） | [MEMORY-SU §规则 11](file:///workspace/.monkeycode/MEMORY-SU.md#规则-11法律合规标准pr2026-07-08-追加) |
| 🔴 **规则 12** | PR | API 必须认证/权限校验，密码强哈希，SQL 参数化，敏感操作审计 | [MEMORY-SU §规则 12](file:///workspace/.monkeycode/MEMORY-SU.md#规则-12法律安全标准pr2026-07-08-追加) |
| 🔴 **规则 13** | PR | 修复按批次连续执行，CI 全绿自动下一批，每批 6-8 文件；**步骤 4 修复后推送前自审**（内容正确性+注释规范性+注释一致性，与规则 20 联动） | [MEMORY-SU §规则 13](file:///workspace/.monkeycode/MEMORY-SU.md#规则-13修复流程自动化与连续执行pr2026-07-11-追加) |
| 🔴 **规则 14** | PR | 禁止 `#[allow(...)]` 警告抑制，所有警告视为错误必须修复 | [MEMORY-SU §规则 14](file:///workspace/.monkeycode/MEMORY-SU.md#规则-14移除所有警告抑制所有警告视为错误pr2026-07-12-追加) |
| 🔴 **规则 15** | PR | 复审按规矩进行，baseline 警告视为错误，8 维度闭环 + 4 轮次状态 | [MEMORY-SU §规则 15](file:///workspace/.monkeycode/MEMORY-SU.md#规则-15复审严格规范--业务财务运行逻辑闭环pr2026-07-13-追加2026-07-17-精简) |
| 🔴 **规则 16** | PR | `.monkeycode/` 全目录强制追踪，禁止忽略任何文件 | [MEMORY-SU §规则 16](file:///workspace/.monkeycode/MEMORY-SU.md#规则-16monkeycode-全目录强制追踪pr2026-07-17-追加) |
| 🔴 **规则 17** | PR | 审计计划/复审规则变更时，5 个关联文档必须同步更新 | [MEMORY-SU §规则 17](file:///workspace/.monkeycode/MEMORY-SU.md#规则-17审计文档同步规则pr2026-07-17-追加) |
| 🔴 **规则 18** | PR | 规则自我迭代机制（四分类 PR/PH/IR/IH + 6 条触发条件 + 自动记录） | [MEMORY-SU §规则 18](file:///workspace/.monkeycode/MEMORY-SU.md#规则-18规则自我迭代机制pr2026-07-17-追加) |
| 🔴 **规则 19** | PR | 工具连接异常分级响应（L1 60s / L2 60-180s / L3 30min 周期）+ 非阻塞推理 | [MEMORY-SU §规则 19](file:///workspace/.monkeycode/MEMORY-SU.md#规则-19工具连接异常重试策略pr2026-07-17-追加2026-07-17-二次迭代增强) |
| 🔴 **规则 20** | PR | 注释必须与功能实现一致，禁止随意编写；含 doc comment/行内/TODO；CI 强制检查 | [MEMORY-SU §规则 20](file:///workspace/.monkeycode/MEMORY-SU.md#规则-20注释与功能一致性强制pr2026-07-17-追加) |

---

## 二、常规规则

- **每项修复 1 commit**：bug 修复按"每项 1 commit"原则，便于回滚和审计
- **公开端点收敛**：仅登录/刷新/健康检查可匿名访问，其他所有端点必须认证
- **多租户已删除**（2026-06-28 m0029）：所有 tenant_id 列/字段/过滤/索引/管理表均已移除
- **CI/CD Only**：禁止本地构建，所有验证走 GitHub Actions（详见规则 13/14）

---

## 三、文件分工

| 文件 | 用途 | 禁止内容 |
|------|------|----------|
| `MEMORY.md` | 规则索引（一句话核心 + 链接） | 详细执行要求、整理记录、批次摘要、历史详情 |
| `MEMORY-SU.md` | 规则详细说明 | 任务相关内容 |
| `doto.md` | 未完成任务（任务队列） | 已完成批次详细表格、历史修复详情 |
| `doto-su.md` | 已完成任务详细记录 | —（接收所有归档内容） |
| `CHANGELOG.md` | 任务一句话总结 | 展开详情、技术要点 |
| `audit_assignment.md` | 审计任务分配和复审规则 | 审计结果详情（保存到 docs/audits/） |

---

## 四、基础规范（摘要）

- **沟通语言**：中文回复，简洁高效，进度可见（TodoWrite），错误透明
- **编码规范**：禁止硬编码，注释必须中文
- **数据库**：PostgreSQL 远程连接模式
- **死代码处理**：不使用的文件/代码必须删除，修改前评估影响范围
- **Bug.md 实时管理**：发现漏洞立即修复，修复后删除条目，保留空文件占位
- **面料行业业务术语（用户 2026-07-17 明确澄清）**：
  - **缸号（batch_no）= 染色批次号**：同一概念，仅叫法不同，指一次染色生产的一个缸的批次号
  - **染色批号（dye_lot_no）**：面料行业 lot 概念，指同产品同颜色不同时间/染缸的批次标识，用于库存/发货/追溯防色差混批
  - **四维标识**：product_id + color_no + dye_lot_no + batch_no（面料追溯核心）
  - 禁止在代码注释/UI 文案中将 dye_lot_no 译为"染色批次号"（与 batch_no 混淆），统一用"染色批号"

> 详细规范见 [MEMORY-SU §三、基础规范](file:///workspace/.monkeycode/MEMORY-SU.md#三基础规范)

---

## 五、CI/CD 强制（摘要）

- **禁止本地编译**：所有验证走 GitHub Actions CI
- **CI 监控 API**：`/commits/{sha}/check-runs` + `/actions/runs/{id}/logs` + `/check-runs/{id}/annotations`
- **服务器**：bingxi-backend systemd，端口 8082，部署命令 `bingxi update`
- **禁止 Docker**：不得创建 Dockerfile、docker-compose.yml

> 详细规范见 [MEMORY-SU §五、CI/CD 强制](file:///workspace/.monkeycode/MEMORY-SU.md#五cicd-强制)

---

## 六、核心经验（摘要）

- **沙箱网络**：22 端口阻断，443 可用；GitHub Token 存 `~/.git-credentials`（600 权限）
- **Clippy Baseline 陷阱**：baseline 只含 `^(warning|error):` 摘要行，单行修改会导致行号偏移触发假警告
- **is_production() 陷阱**：只读 APP_ENV 环境变量，不读 config.yaml env 字段
- **systemd EnvironmentFile 一致性**：deploy 脚本 CONFIG_DIR 必须与 service 文件路径一致
- **SeaORM Trait 必导**：find/filter/column/count 各需对应 trait，清理时逐个静态验证

> 详细经验见 [MEMORY-SU §六、核心经验](file:///workspace/.monkeycode/MEMORY-SU.md#六核心经验关键排错与开发经验)

---

## 七、归档索引

- 完整历史整理前内容：`.monkeycode/docs/archives/2026-07-13/` 等子目录
- 历史审计报告：`.monkeycode/docs/audits/`（v5-v15）
- V15 审计报告：`.monkeycode/docs/audits/v15/`（21 批 + 汇总）

---

## 八、规则自我迭代日志（索引）

> 详细迭代日志见 [MEMORY-SU §十一、规则自我迭代日志](file:///workspace/.monkeycode/MEMORY-SU.md#十一规则自我迭代日志)
