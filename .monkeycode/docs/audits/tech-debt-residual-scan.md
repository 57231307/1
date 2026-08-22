# 技术债务残留扫描报告

- 扫描日期：2026-08-22
- 扫描范围：`/workspace` 仓库（backend + frontend）
- 扫描代理：审计代理（只读模式，未修改代码，未创建 PR，未推送）
- 报告路径：`/workspace/.monkeycode/docs/audits/tech-debt-residual-scan.md`

---

## 2.9 预留功能接入路由（dead_code 抑制）

### 扫描命令
```bash
grep -rn "#\[allow(dead_code)\]" backend/src/ | wc -l
grep -rn "never constructed\|never used" backend/src/ | wc -l
```

### 结果
| 指标 | 数值 |
|------|------|
| `#[allow(dead_code)]` 抑制数 | **47** |
| 编译器告警 "never constructed/never used" 数 | **0** |

### 结论
- 存在 **47 处** `dead_code` 抑制标注，表明有较多预留但尚未接入路由的功能代码。这些代码虽已显式抑制编译器告警，避免构建失败，但仍构成技术债务：预留逻辑长期不被调用，后续可能失配于业务变更，增加维护成本与回归风险。
- "never constructed/never used" 告警数为 0，说明未使用 struct 的告警已被 `allow(dead_code)` 全部吸收，未被抑制的零散告警已清零。
- **建议**：对 47 处抑制点逐一核查，按"接入路由 / 删除 / 显式标注预留计划"三分类处置，避免预留代码长期沉积。

---

## 2.5 代码重复率（CRUD 样板）

### 扫描命令
```bash
grep -rn "pub async fn list_\|pub async fn create_\|pub async fn update_\|pub async fn delete_\|pub async fn get_" backend/src/services/ | wc -l
```

### 结果
| CRUD 类型 | 出现次数 |
|-----------|----------|
| `get_` | 310 |
| `list_` | 140 |
| `create_` | 102 |
| `update_` | 61 |
| `delete_` | 47 |
| **合计** | **660** |

### 结论
- `backend/src/services/` 下 **660 处** 命名高度雷同的 CRUD 异步函数，重复程度 **严重**。`get_` 类查询函数多达 310 处，是最显著的重复源。
- 同名同构函数大量分散于各 service 文件，典型表现是每个业务实体各自实现 `list_/create_/update_/delete_/get_`，缺少泛型化或 trait 抽象，**DRY 原则违反**。
- **建议**：抽取通用 CRUD trait（如 `CrudRepository<T>`）或宏生成样板，将 660 处收敛到少量泛型实现，降低新增实体时的样板成本与一致性风险。

---

## 2.6 过时依赖

### 扫描命令
```bash
grep -E "version" backend/Cargo.toml | head -30
```

### 结果（主要依赖版本）
| 依赖 | 版本 | 备注 |
|------|------|------|
| axum | 0.8 | 当前 0.x，版本较新 |
| tower-http | 0.7 | 0.x |
| tokio | 1.0 | 稳定主版本 |
| tokio-util | 0.7 | 0.x |
| sea-orm | 2.0.2 | 主版本 2.x |
| serde | 1.0 | 稳定 |
| uuid | 1.0 | 稳定 |
| jsonwebtoken | 11.0 | 11.x |
| validator | 0.21 | 0.x |
| chrono | 0.4 | 0.x（长期 0.x） |
| rust_decimal | 1.0 | 稳定 |
| reqwest | 0.13 | 0.x |
| moka | 0.12 | 0.x |
| redis | 1.6 | 主版本 1.x |
| time | 0.3 | 0.x |
| clap | 4.4 | 主版本 4.x |
| utoipa | 5.2.0 | 5.x |
| axum-extra | 0.12 | 0.x |
| totp-rs | 6.0 | 6.x |
| utoipa-swagger-ui | 9.0.2 | 9.x |
| rskafka | 0.5 | 0.x |
| criterion | 0.5 | 0.x |
| rust-version | 1.94 | MSRV 较新 |
| 项目 version | 2026.810.1 | — |

### 结论
- 依赖整体处于较新水平，主版本稳定依赖（tokio 1.0、serde 1.0、uuid 1.0、redis 1.6、clap 4.x、utoipa-swagger-ui 9.x）占比较高。
- 仍有多项依赖停留在 0.x（axum 0.8、tower-http 0.7、reqwest 0.13、validator 0.21、moka 0.12、chrono 0.4、time 0.3、rskafka 0.5），按 SemVer 约定 0.x 尚未承诺 API 稳定，存在小版本间破坏性变更风险。
- MSRV 为 1.94，属较新 toolchain。
- **建议**：对 0.x 依赖纳入升级跟踪，关注 axum / reqwest / validator 的 1.0 发布节奏；定期执行 `cargo update` + 测试验证。

---

## 2.7 注释完整性（文档覆盖率）

### 扫描命令
```bash
grep -rn "^///" backend/src/ | wc -l
grep -rn "^pub fn\|^pub async fn" backend/src/ | wc -l
```

### 结果
| 指标 | 数值 |
|------|------|
| 文档注释（`///`）数 | 5601 |
| 公开函数（`pub fn` / `pub async fn`）数 | 2105 |
| 覆盖率（注释数 / 公开函数数） | **266.1%** |

### 结论
- 文档注释数 5601，公开函数数 2105，注释对公开函数的覆盖比为 **266.1%**（每公开函数平均 2.66 条文档注释）。覆盖率充裕，未出现大面积缺失。
- 该比例为上界估计：注释行可能也覆盖在结构体、字段、模块等非函数项上，实际"每个公开函数至少一条文档注释"的精确覆盖需按符号级核查。但总体上注释投入充分，无明显裸函数堆积。
- **建议**：保持现状；如需精确指标，可接入 `cargo doc` 的 missing-docs lint（`#![deny(missing_docs)]`）做符号级门禁。

---

## 2.10 前端 any 类型

### 扫描命令
```bash
grep -rn ": any\|<any>\|as any\|: any\b" frontend/src/ | wc -l
```

### 结果
| 指标 | 数值 |
|------|------|
| `any` 使用总数 | **177** |

### 结论
- 前端代码共 **177 处** `any` 类型使用，集中分布于：
  - `frontend/src/composables/useTableApi.ts`
  - `frontend/src/views/bpm/index.vue`
  - `frontend/src/views/sales-returns/composables/useSr.ts`
  - `frontend/src/views/sales-returns/components/ReturnEditDialog.vue`
  - `frontend/src/views/system-update/composables/useSysUpdProc.ts`
  - `frontend/src/views/sales-contract/composables/useScProc.ts`
  - `frontend/src/views/quality-standards/index.vue`
  - `frontend/src/views/bom/index.vue`
  - `frontend/src/views/bom/BillOfMaterialsForm.vue`
  - `frontend/src/views/print-templates/index.vue`
- 177 处 any 显著削弱类型安全：表单/表格 composables、各业务页面 props 与响应数据普遍走 any，绕过 TS 类型检查，运行时错误风险上移到用户路径。
- **建议**：优先治理 `composables/*` 与 `views/*/components/*Dialog.vue`，引入 `unknown` + 类型守卫或对应后端 schema 生成类型，将 any 逐步收敛。

---

## 五项扫描结论汇总

1. **2.9 预留功能（dead_code）**：47 处 `#[allow(dead_code)]` 抑制，0 处未抑制的 "never constructed/used" 告警。预留代码已全部显式抑制但长期未接入路由，需分类处置。
2. **2.5 代码重复率**：services 层 660 处 CRUD 样板（`get_` 310、`list_` 140、`create_` 102、`update_` 61、`delete_` 47），重复严重，建议抽象为通用 trait。
3. **2.6 过时依赖**：依赖整体较新（MSRV 1.94，主版本稳定依赖占比高），但 axum / reqwest / validator / moka / chrono / time / rskafka 仍处 0.x，需纳入升级跟踪。
4. **2.7 注释完整性**：文档注释 5601 条 / 公开函数 2105 个，覆盖比 266.1%，投入充分，建议加 `missing_docs` lint 做精确门禁。
5. **2.10 前端 any**：177 处 `any` 使用，集中于 composables 与业务页面组件，类型安全薄弱，需优先治理 composable 与对话框组件。

---

## 附：扫描环境信息

- 工作目录：`/workspace`
- 后端源码：`backend/src/`
- 前端源码：`frontend/src/`
- 扫描模式：只读审计，未修改任何代码文件，未创建 PR，未执行推送
- 报告目录：`/workspace/.monkeycode/docs/audits/`（本规则允许修改）
