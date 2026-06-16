# 色卡仓储管理模块实施 Plan

> P0-4 行业功能：色卡仓储管理
> 计划日期: 2026-06-17
> 关联 spec: `docs/superpowers/specs/2026-06-16-color-card-design.md`

---

## 一、任务总览

**总任务数**：14 Task（Week 1: 5 / Week 2: 5 / Week 3: 4）

**实施模式**：subagent-driven，3 周分批（本周由 P0-4 单代理执行）

**目标**：完成 spec → migration → entity → DTO → service → handler → route → frontend → test → docs → TEST 版本全流程

---

## 二、Week 1（5 Task）— 数据层 + 基础服务

### Task 1.1：色卡仓储管理 spec + plan
- **产出**：
  - `docs/superpowers/specs/2026-06-16-color-card-design.md`（已完成）
  - `docs/superpowers/plans/2026-06-16-color-card-plan.md`（本文件）
- **commit**：`docs(spec): 色卡仓储管理模块设计 spec + 实施 plan`
- **预估**：0.5h

### Task 1.2：3 张表 migration
- **产出**：
  - `backend/migrations/20260617000006_create_color_cards/up.sql` + `down.sql`
  - `backend/migrations/20260617000007_create_color_card_items/up.sql` + `down.sql`
  - `backend/migrations/20260617000008_create_color_card_borrow_records/up.sql` + `down.sql`
- **表设计**：见 spec §3
- **commit**：`feat(db): 色卡仓储管理 3 张表 migration`
- **预估**：1h

### Task 1.3：3 个 entity + 3 个 DTO
- **产出**：
  - `backend/src/models/color_card.rs`（entity）
  - `backend/src/models/color_card_item.rs`（entity）
  - `backend/src/models/color_card_borrow_record.rs`（entity）
  - `backend/src/models/color_card_create_dto.rs`（CreateColorCardDto / UpdateColorCardDto）
  - `backend/src/models/color_card_item_dto.rs`（CreateColorItemDto / BatchImportDto）
  - `backend/src/models/color_card_borrow_dto.rs`（BorrowDto / ReturnDto / LostDto）
  - `backend/src/models/color_card_response_dto.rs`（列表 / 详情 / 扫码响应）
- **文件 #![allow(dead_code)]**：3 entity 文件保留（SeaORM 派生宏使用）
- **DTO 文件**：不加 `#![allow(dead_code)]`（由项目死代码规范要求）
- **commit**：`feat(model): 色卡仓储管理 3 entity + 3 DTO`
- **预估**：1.5h

### Task 1.4：CIELab 色彩空间转换工具
- **产出**：
  - `backend/src/utils/color_space_converter.rs`（RGB ↔ CMYK ↔ Lab ↔ HEX 转换 + ΔE 色差 + 5 单元测试）
- **算法**：
  - `rgb_to_hex(r, g, b) -> String`（自动补 `#`）
  - `hex_to_rgb(hex: &str) -> Result<(u8, u8, u8), String>`（校验格式）
  - `rgb_to_cmyk(r, g, b) -> (u8, u8, u8, u8)`（0-100 百分比）
  - `rgb_to_lab(r, g, b) -> (f64, f64, f64)`（D65 参考白点 + sRGB 转换矩阵）
  - `delta_e_76(lab1, lab2) -> f64`（CIELab 1976 色差公式）
  - `delta_e_is_acceptable(delta_e) -> bool`（阈值 3.0）
- **commit**：`feat(util): CIELab 色彩空间转换工具（RGB/CMYK/Lab/HEX/ΔE）`
- **预估**：1h

### Task 1.5：色卡 CRUD service + 色号 service 骨架
- **产出**：
  - `backend/src/services/color_card_crud_service.rs`（create / list / get / update / archive）
  - `backend/src/services/color_card_item_service.rs`（list / create / update / delete / batch_import）
- **校验**：
  - 多租户隔离（强制 tenant_id 过滤）
  - 编号唯一（依赖 DB UNIQUE）
  - 色号编码唯一（依赖 DB UNIQUE）
  - RGB / Lab 范围校验
- **commit**：`feat(service): 色卡 CRUD + 色号 service（CRUD + 批量导入）`
- **预估**：2h

---

## 三、Week 2（5 Task）— 借出服务 + handler + route

### Task 2.1：借出管理 service
- **产出**：
  - `backend/src/services/color_card_borrow_service.rs`（borrow / return / mark_lost / mark_damaged / list_records）
- **状态机**：
  - borrowed → returned / lost / damaged
  - 终态不可再转换
- **commit**：`feat(service): 色卡借出管理 service（借出/归还/遗失/历史）`
- **预估**：1.5h

### Task 2.2：扫码查询 service + 导出 service
- **产出**：
  - `backend/src/services/color_card_scan_service.rs`（scan_by_code / scan_by_id，返回 RGB/CMYK/Lab/配方/价格）
  - 导出 CSV 工具函数（在 handler 层直接实现，service 不单独计）
- **commit**：`feat(service): 色卡扫码查询 service（RGB/CMYK/Lab/配方/价格）`
- **预估**：1h

### Task 2.3：13 handler 实现
- **产出**：
  - `backend/src/handlers/color_card_handler.rs`（13 handler）
  - 分类：
    - CRUD 5：list / get / create / update / archive
    - 色号 4：list_items / create_item / update_item / delete_item
    - 借出 4：borrow / return / mark_lost / list_records
    - 扫码导出 2：scan_code / batch_import_items
  - 注：批量导入与导出合并到 batch endpoint（POST + GET 同路径不同方法）
- **commit**：`feat(handler): 色卡仓储管理 13 handler`
- **预估**：2h

### Task 2.4：16 路由 + 集成到 lib.rs
- **产出**：
  - `backend/src/routes/color_card.rs`（16 路由）
  - `backend/src/routes/mod.rs`（添加 `pub mod color_card;` + `.nest("/api/v1/erp/color-cards", color_card::routes())`）
  - `backend/src/models/mod.rs`（添加 6 个新模型声明）
  - `backend/src/services/mod.rs`（添加 4 个新服务声明）
  - `backend/src/handlers/mod.rs`（添加 `pub mod color_card_handler;`）
  - `backend/src/utils/app_state.rs`（添加 4 个 service 字段）
- **commit**：`feat(route): 色卡仓储管理 16 路由 + AppState 集成`
- **预估**：1.5h

### Task 2.5：5 集成测试
- **产出**：
  - `backend/tests/color_card_crud_test.rs`（色卡 CRUD + 多租户隔离）
  - `backend/tests/color_card_item_test.rs`（色号 CRUD + 批量导入）
  - `backend/tests/color_card_borrow_test.rs`（借出/归还/遗失流程）
  - `backend/tests/color_card_scan_test.rs`（扫码查询 + 色彩空间转换）
  - `backend/tests/color_card_e2e_test.rs`（端到端流程）
- **commit**：`test: 色卡仓储管理 5 集成测试`
- **预估**：2h

---

## 四、Week 3（4 Task）— 前端 + 文档 + 交付

### Task 3.1：4 前端页面 + 3 组件
- **产出**：
  - `frontend/src/views/color-cards/list.vue`（列表 + 搜索 + 筛选 + 新建）
  - `frontend/src/views/color-cards/create.vue`（表单 + 实时预览）
  - `frontend/src/views/color-cards/detail.vue`（基本信息 + 色号宫格 + 借出时间线）
  - `frontend/src/views/color-cards/borrow.vue`（借出/归还/遗失 + 历史）
  - `frontend/src/components/ColorCardGrid.vue`（色号宫格）
  - `frontend/src/components/ColorItemEditor.vue`（色号编辑器）
  - `frontend/src/components/BorrowRecordTimeline.vue`（借出时间线）
  - `frontend/src/router/` 添加 4 个路由
- **commit**：`feat(frontend): 色卡仓储管理 4 页面 + 3 组件`
- **预估**：3h

### Task 3.2：API 客户端 + E2E 测试
- **产出**：
  - `frontend/src/api/color-card.ts`（16 端点 TypeScript 函数 + 类型）
  - `frontend/e2e/color-card.spec.ts`（Playwright E2E：列表 → 创建 → 借出 → 归还）
- **commit**：`feat(frontend): 色卡 API 客户端 + E2E 测试`
- **预估**：1.5h

### Task 3.3：用户手册 + API 文档
- **产出**：
  - `docs/color-card-user-manual.md`（用户操作手册，含 19 个测试场景）
  - `docs/color-card-api.md`（16 端点 API 文档，含 curl 示例 + 响应）
- **commit**：`docs: 色卡仓储管理用户手册 + API 文档`
- **预估**：1.5h

### Task 3.4：TEST 测试版本交付
- **产出**：
  - `dist/test-version-P0-4/Dockerfile`
  - `dist/test-version-P0-4/docker-compose.yml`
  - `dist/test-version-P0-4/start.sh` / `stop.sh`
  - `dist/test-version-P0-4/config/color-card.toml.example`
  - `dist/test-version-P0-4/README.md`
  - `dist/test-version-P0-4/test-scenarios.md`（19 个测试场景）
  - `docs/color-card-deployment-guide.md`
- **commit**：`docs(dist): P0-4 TEST 测试版本交付 + 部署指南`
- **预估**：1.5h

---

## 五、关键路径与风险

| 风险 | 影响 | 应对 |
|------|------|------|
| `cargo check` 报错 | 中 | 立即修复（按 P0-3 模式） |
| `cargo test` OOM | 中 | 跳过测试，依赖 CI 验证 |
| SeaORM entity 字段错位 | 高 | 严格按 spec 字段顺序 |
| AppState 未更新导致编译失败 | 高 | 同步 4 处（models / services / handlers / app_state） |
| 路由未挂载 | 中 | 同时更新 `routes/mod.rs` + `lib.rs` |
| 前端类型定义不一致 | 中 | 严格按 DTO 字段命名 |

---

## 六、最终交付清单

### 后端（25 文件）
- 3 migration (up + down) = 6 文件
- 3 entity + 6 DTO = 9 文件
- 4 service = 4 文件
- 1 handler = 1 文件
- 1 route = 1 文件
- 1 util = 1 文件
- 5 集成测试 = 5 文件（共享 `tests/` 目录）
- 文档与修改：~6 处
- **合计**：~30 文件

### 前端（8 文件）
- 4 页面 = 4 文件
- 3 组件 = 3 文件
- 1 API 客户端 = 1 文件
- 1 E2E = 1 文件
- router 修改 = 1 处
- **合计**：9 文件

### 文档（5 文件）
- spec + plan = 2 文件
- user-manual + api + deployment = 3 文件

### 交付物（7 文件）
- Dockerfile + docker-compose + start/stop + config + README + test-scenarios = 6 文件
- 部署指南（在 docs 下）= 1 文件

---

## 七、PR 计划

- **分支**：`trae/solo-agent-P0-4-color-card`
- **目标分支**：`test`
- **PR title**：`feat(color-card): 色卡仓储管理（3 表 + 16 端点 + 4 页面 + 3 组件 + E2E + 测试版本）`
- **commit 数**：约 9 commit
- **merge 方式**：squash merge
- **merge commit**：预计 1 commit 到 test 分支
- **merge 后**：删除 `trae/solo-agent-P0-4-color-card` 分支
- **main 分支**：不动

---

> 文档版本：v1.0 | 2026-06-17 | P0-4 色卡仓储管理模块
