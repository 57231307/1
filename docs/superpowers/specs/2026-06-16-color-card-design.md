# 色卡仓储管理模块设计 Spec

> 冰西 ERP P0-4 行业功能：色卡仓储管理（纺织行业色卡生命周期与借出跟踪）
> 设计日期: 2026-06-17
> 关联代码: `backend/src/{models,services,handlers,routes}/color_card*` + 前端 `frontend/src/views/color-cards/` + 测试版本 `dist/test-version-P0-4/`

---

## 1. 业务背景

### 1.1 行业定位

色卡（color card / swatch）是纺织行业客户选色、定样、确认色号的核心物料。1 张色卡包含 50-500 个色号（color code），每张色卡对应：

- 一组色号编码（按 PANTONE / CNCS / 自定义规则）
- 一组色彩空间坐标（RGB / CMYK / CIELab）
- 一套存储位置（色卡盒、档案柜）
- 一组借出历史（客户、员工、时间、用途）
- 一组关联业务（配方、价格、订单）

### 1.2 行业标准

- **GB/T 26377-2022** 纺织品颜色标准样品技术规范（色卡管理基础）
- **PANTONE** 国际通用色卡（TPG / TPX / TCX 多种表面）
- **CNCS（中国颜色体系）** GB/T 15608-2006
- **ΔE 色差** ≤ 3（CIELab 色差行业标准，用于打样合格判定）
- **二维码 / RFID** 每个色号独立标识，扫码即可查色号详情

### 1.3 业务痛点

| 痛点 | 当前 | 目标 |
|------|------|------|
| 色卡实物管理 | Excel + 手工登记 | 数字化色卡库 + 借出跟踪 |
| 色号查询 | 翻实体色卡 | 扫码查色号（RGB/CMYK/Lab/配方/历史）|
| 借出 / 归还 | 纸质登记 | 系统流程，状态可追溯 |
| 遗失赔付 | 凭记忆 | 流程化登记 + 自动金额计算 |
| 客户专属色卡 | 与通用色卡混杂 | 标签分类 + 权限隔离 |

---

## 2. 范围与目标

### 2.1 范围（In Scope）

- 3 张核心表（`color_cards` / `color_card_items` / `color_card_borrow_records`）
- 3 个 entity + 3 个 DTO + 16 个 API 端点 + 4 个 service + 13 个 handler
- 4 个前端页面（list / create / detail / borrow-management）
- 3 个前端组件（ColorCardGrid / ColorItemEditor / BorrowRecordTimeline）
- CIELab 色彩空间转换工具（RGB ↔ Lab）
- 色号批量导入（CSV / Excel）
- 集成测试 + E2E 测试
- 用户手册 + API 文档 + 部署指南
- TEST 测试版本交付（Docker + docker-compose）

### 2.2 不在范围（Out of Scope）

- PANTONE / CNCS 官方色彩库对接（仅支持编码引用，不做实时反查）
- 色卡印刷输出 PDF（仅支持封面图上传）
- 客户线上选色交互页面（仅内部管理，不做 C 端）
- RFID 设备集成（仅二维码）
- 多语言（仅简体中文）

### 2.3 验收目标

- 16 个 API 端点全部实现并通过集成测试
- 4 个前端页面 + 3 个组件可正常访问
- 色号批量导入 1000 条 < 5 秒
- 借出 / 归还 / 遗失赔付流程闭环
- 多租户隔离（`extract_tenant_id` 强制）
- TEST 测试版本可在 Docker 中启动

---

## 3. 数据模型

### 3.1 ER 关系

```
color_cards (1) ──< (N) color_card_items
color_cards (1) ──< (N) color_card_borrow_records
customers (1) ──< (N) color_card_borrow_records
users (1) ──< (N) color_card_borrow_records (borrowed_by)
dye_recipes (1) ──< (N) color_card_items (dye_recipe_id)
product_color_prices (1) ──< (N) color_card_items (product_color_price_id)
```

### 3.2 color_cards（色卡主表）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 主键 |
| card_no | VARCHAR(50) | UNIQUE NOT NULL | 色卡编号（如 `PANTONE-TPX-2024-SS`） |
| card_name | VARCHAR(200) | NOT NULL | 色卡名称 |
| card_type | VARCHAR(50) | NOT NULL | 类型（PANTONE / CNCS / CUSTOM） |
| season | VARCHAR(20) | | 季节（2024SS / 2024AW / 经典） |
| brand | VARCHAR(100) | | 品牌（自有 / 客户定制） |
| total_colors | INT | NOT NULL DEFAULT 0 | 色号总数 |
| status | VARCHAR(20) | NOT NULL DEFAULT 'active' | 状态（active / archived / lost） |
| description | TEXT | | 描述 |
| cover_image_url | TEXT | | 封面图 URL |
| tenant_id | BIGINT | NOT NULL | 多租户隔离 |
| created_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | 更新时间 |

**约束**：`status IN ('active', 'archived', 'lost')`

**索引**：
- `idx_color_cards_tenant` (tenant_id)
- `idx_color_cards_card_no` (card_no)
- `idx_color_cards_status` (status)
- `idx_color_cards_type_season` (card_type, season)

### 3.3 color_card_items（色卡明细）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 主键 |
| color_card_id | BIGINT | FK → color_cards, NOT NULL | 所属色卡 |
| color_code | VARCHAR(50) | NOT NULL | 色号编码（如 `18-1664 TPX`） |
| color_name | VARCHAR(200) | NOT NULL | 色号名称 |
| rgb_r | INT | NOT NULL CHECK 0..255 | RGB 红 |
| rgb_g | INT | NOT NULL CHECK 0..255 | RGB 绿 |
| rgb_b | INT | NOT NULL CHECK 0..255 | RGB 蓝 |
| cmyk_c | DECIMAL(5,2) | | CMYK 青（0-100） |
| cmyk_m | DECIMAL(5,2) | | CMYK 品红 |
| cmyk_y | DECIMAL(5,2) | | CMYK 黄 |
| cmyk_k | DECIMAL(5,2) | | CMYK 黑 |
| lab_l | DECIMAL(6,2) | | CIELab L (0-100) |
| lab_a | DECIMAL(6,2) | | CIELab a (-128..127) |
| lab_b | DECIMAL(6,2) | | CIELab b (-128..127) |
| pantone_code | VARCHAR(50) | | PANTONE 编码 |
| cncs_code | VARCHAR(50) | | CNCS 编码 |
| custom_code | VARCHAR(50) | | 自定义编码 |
| hex_value | VARCHAR(7) | NOT NULL | `#RRGGBB` |
| dye_recipe_id | BIGINT | FK → dye_recipes | 关联染色配方 |
| product_color_price_id | BIGINT | FK → product_color_prices | 关联色号价格 |
| swatch_image_url | TEXT | | 色片图 URL |
| sequence | INT | NOT NULL DEFAULT 0 | 在色卡中的顺序 |
| tenant_id | BIGINT | NOT NULL | 多租户隔离 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

**唯一约束**：`UNIQUE (color_card_id, color_code)`（同一色卡内色号编码唯一）

**索引**：
- `idx_color_items_card` (color_card_id)
- `idx_color_items_code` (color_code)
- `idx_color_items_pantone` (pantone_code) WHERE pantone_code IS NOT NULL
- `idx_color_items_cncs` (cncs_code) WHERE cncs_code IS NOT NULL
- `idx_color_items_tenant` (tenant_id)

### 3.4 color_card_borrow_records（借出记录）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 主键 |
| color_card_id | BIGINT | FK → color_cards, NOT NULL | 色卡 |
| customer_id | BIGINT | FK → customers, NOT NULL | 借出客户 |
| borrowed_by | BIGINT | FK → users, NOT NULL | 经办人（员工） |
| borrowed_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | 借出时间 |
| expected_return_at | TIMESTAMPTZ | | 预计归还时间 |
| actual_return_at | TIMESTAMPTZ | | 实际归还时间 |
| status | VARCHAR(20) | NOT NULL DEFAULT 'borrowed' | 状态（borrowed / returned / lost / damaged） |
| purpose | TEXT | | 用途（开发选色 / 客户确认 / 展会展示） |
| notes | TEXT | | 备注 |
| compensation_amount | DECIMAL(15,2) | | 遗失赔付金额 |
| tenant_id | BIGINT | NOT NULL | 多租户隔离 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

**约束**：`status IN ('borrowed', 'returned', 'lost', 'damaged')`

**索引**：
- `idx_borrow_card` (color_card_id)
- `idx_borrow_customer` (customer_id)
- `idx_borrow_status` (status)
- `idx_borrow_tenant` (tenant_id)
- `idx_borrow_borrowed_at` (borrowed_at DESC)

---

## 4. 后端架构

### 4.1 4 个 Service

| Service | 职责 | 端点 |
|---------|------|------|
| `ColorCardCrudService` | 色卡 CRUD | list / get / create / update / archive |
| `ColorCardItemService` | 色号 CRUD + 批量导入 | list / create / update / delete / import / export |
| `ColorCardBorrowService` | 借出 / 归还 / 遗失 / 历史 | borrow / return / mark_lost / list_records |
| `ColorCardScanService` | 扫码查询色号详情 | scan_by_code / scan_by_id |

### 4.2 16 个 API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/erp/color-cards` | GET | 色卡列表（分页 + 多条件） |
| `/api/v1/erp/color-cards` | POST | 创建色卡 |
| `/api/v1/erp/color-cards/:id` | GET | 色卡详情（含色号列表） |
| `/api/v1/erp/color-cards/:id` | PUT | 更新色卡 |
| `/api/v1/erp/color-cards/:id` | DELETE | 归档色卡（soft delete） |
| `/api/v1/erp/color-cards/:id/items` | GET | 色号列表 |
| `/api/v1/erp/color-cards/:id/items` | POST | 新增色号 |
| `/api/v1/erp/color-cards/:id/items/batch` | POST | 批量导入色号（CSV/Excel） |
| `/api/v1/erp/color-cards/:id/items/:item_id` | PUT | 更新色号 |
| `/api/v1/erp/color-cards/:id/items/:item_id` | DELETE | 删除色号 |
| `/api/v1/erp/color-cards/borrow` | POST | 借出色卡 |
| `/api/v1/erp/color-cards/return/:record_id` | POST | 归还色卡 |
| `/api/v1/erp/color-cards/lost/:record_id` | POST | 登记遗失（含赔付金额） |
| `/api/v1/erp/color-cards/borrow-records` | GET | 借出历史（按客户/状态/时间） |
| `/api/v1/erp/color-cards/scan/:code` | GET | 扫码查询色号详情（RGB/CMYK/Lab/配方） |
| `/api/v1/erp/color-cards/export/:id` | GET | 导出色卡（CSV） |

### 4.3 13 个 Handler（按 P0-3 模式）

| 分类 | Handler |
|------|---------|
| CRUD 5 | list_color_cards / get_color_card / create_color_card / update_color_card / archive_color_card |
| 色号 5 | list_color_items / create_color_item / update_color_item / delete_color_item / batch_import_items |
| 借出 4 | borrow_color_card / return_color_card / mark_lost_color_card / list_borrow_records |
| 扫码 2 | scan_color_code / export_color_card |

注：批量导入、扫码查询、导出在 handler 层调用对应 service 方法（不单独计 handler，重复利用 create_color_item / list_color_items 等）。

实际 handler 数为：**5 (CRUD) + 4 (色号) + 4 (借出) + 2 (扫码导出) = 15 handler**。本模块 13 handler 的目标是：色号批量导入与色号导出合并为 1 个（共用 batch endpoint），最终实际为 13 handler。

### 4.4 状态机

借出记录状态机：
```
borrowed ──(return)──→ returned
   │
   ├──(mark_lost)──→ lost
   │
   └──(mark_damaged)──→ damaged
```

`returned` / `lost` / `damaged` 为终态，不可再转换。

---

## 5. 前端架构

### 5.1 4 个页面

| 页面 | 路径 | 说明 |
|------|------|------|
| 列表页 | `/color-cards/list` | 列表 + 搜索 + 筛选 + 新建入口 |
| 创建页 | `/color-cards/create` | 表单 + 实时预览色号 |
| 详情页 | `/color-cards/detail/:id` | 基本信息 + 色号宫格 + 借出时间线 |
| 借出管理 | `/color-cards/borrow` | 借出 / 归还 / 遗失登记 + 历史查询 |

### 5.2 3 个组件

| 组件 | 职责 |
|------|------|
| `ColorCardGrid` | 色号宫格展示（按 sequence 排序，hover 显示 RGB/CMYK/Lab） |
| `ColorItemEditor` | 色号编辑（颜色选择器 + RGB/CMYK/Lab 实时联动） |
| `BorrowRecordTimeline` | 借出记录时间线（按时间倒序） |

### 5.3 API 客户端

`frontend/src/api/color-card.ts` — 16 个端点对应的 TypeScript 函数 + 类型定义。

---

## 6. 业务流程

### 6.1 色卡创建流程

1. 用户填写色卡基本信息（编号、名称、类型、季节、品牌）
2. 用户上传封面图（可选）
3. 系统生成色卡主记录
4. 用户切换到「色号管理」Tab
5. 用户选择批量导入（CSV / Excel）或逐个添加
6. 导入时自动转换 RGB → CMYK → Lab（CIELab）
7. 导入完成后更新色卡 `total_colors` 字段

### 6.2 色号批量导入

1. 前端选择 CSV / Excel 文件
2. 调用 `POST /color-cards/:id/items/batch` 上传
3. 后端解析文件，逐行写入 `color_card_items` 表
4. 同步计算每个色号的 CMYK 与 Lab 值
5. 更新 `color_cards.total_colors`
6. 返回导入结果（成功 / 失败条数 + 错误明细）

### 6.3 借出流程

1. 用户选择色卡 + 客户
2. 选择经办人（默认当前用户）
3. 填写用途 + 预计归还时间
4. 提交后创建 `color_card_borrow_records`（status=borrowed）
5. 更新色卡状态（active → borrowed 标记，可选）

### 6.4 归还流程

1. 借出管理页面显示当前借出列表
2. 用户点击「归还」
3. 填写实际归还时间 + 备注
4. 更新记录（status=returned, actual_return_at=now）

### 6.5 遗失赔付流程

1. 用户点击「登记遗失」
2. 填写遗失时间 + 赔付金额（必须 > 0）
3. 更新记录（status=lost, compensation_amount=xxx）
4. 更新色卡状态（active → lost）
5. 写入审计日志（金额变更）

---

## 7. 行业规则覆盖

| 规则 | 校验位置 | 错误信息 |
|------|----------|----------|
| RGB 值范围 0-255 | 数据库 CHECK + service 校验 | 「RGB 值必须为 0-255 整数」 |
| CIELab L 范围 0-100 | service 校验 | 「L 值必须为 0-100」 |
| CIELab a/b 范围 -128~127 | service 校验 | 「a/b 值范围 -128~127」 |
| hex 格式 `#RRGGBB` | service 正则 | 「hex 值必须为 #RRGGBB 格式」 |
| ΔE 色差 ≤ 3（打样合格） | 工具函数 | 返回色差数值 + 是否合格 |
| 色卡编号唯一 | 数据库 UNIQUE | 「色卡编号已存在」 |
| 色卡内色号编码唯一 | 数据库 UNIQUE | 「同一色卡内色号编码已存在」 |
| 借出时间 ≤ 预计归还 + 30 天 | service 校验 | 「预计归还时间不能超过借出时间 + 30 天」 |

---

## 8. 测试策略

### 8.1 集成测试（5 个）

| 测试文件 | 覆盖范围 |
|----------|----------|
| `color_card_crud_test.rs` | 色卡 CRUD + 多租户隔离 |
| `color_card_item_test.rs` | 色号 CRUD + 批量导入 |
| `color_card_borrow_test.rs` | 借出 / 归还 / 遗失流程 |
| `color_card_scan_test.rs` | 扫码查询 + 色彩空间转换 |
| `color_card_e2e_test.rs` | 端到端（创建 → 导入 → 借出 → 归还） |

### 8.2 单元测试（color_space_converter）

- `rgb_to_hex` 边界值
- `hex_to_rgb` 异常输入
- `rgb_to_cmyk` 黑色 / 白色边界
- `rgb_to_lab` D65 参考白点
- `delta_e_76` 色差计算（已知样本对照）
- 4-5 个 unit test

### 8.3 E2E 测试

- `frontend/e2e/color-card.spec.ts`（Playwright）
- 覆盖：列表 → 创建 → 详情 → 借出 → 归还

---

## 9. 部署

### 9.1 部署位置

- 后端：`/api/v1/erp/color-cards/*` 端点（无需新增 nginx 路由）
- 前端：`/color-cards/*` 页面（vue-router 新增）
- 数据库：3 张新表（migration 自动执行）
- 配置：`config/color-card.toml.example`（多租户配置 + 导入文件大小限制）

### 9.2 TEST 测试版本交付

- 路径：`/workspace/dist/test-version-P0-4/`
- 包含：
  - `Dockerfile`（基于后端镜像）
  - `docker-compose.yml`（后端 + PostgreSQL + Redis）
  - `start.sh`（启动脚本）
  - `config/color-card.toml.example`（配置示例）
  - `README.md`（快速启动）
  - `test-scenarios.md`（19 个测试场景）

---

## 10. 验收标准

- [ ] 3 张表 migration 成功（执行 `sea-orm-cli migrate up`）
- [ ] 16 个 API 端点全部实现（含 `swagger` 注释）
- [ ] 13 个 handler 全部实现
- [ ] 4 个 service 全部实现
- [ ] 色号批量导入支持 CSV / Excel（1000 条 < 5 秒）
- [ ] CIELab 色彩空间转换（RGB ↔ Lab）功能正确
- [ ] 借出 / 归还 / 遗失流程闭环
- [ ] 扫码查询色号详情（RGB/CMYK/Lab/配方/价格）
- [ ] 4 个前端页面（list / create / detail / borrow-management）
- [ ] 3 个组件（ColorCardGrid / ColorItemEditor / BorrowRecordTimeline）
- [ ] 5 个集成测试通过
- [ ] 1 个 E2E 测试通过
- [ ] 多租户隔离（`extract_tenant_id` 强制）
- [ ] 用户手册 + API 文档 + 部署指南
- [ ] TEST 测试版本可在 Docker 中启动
- [ ] PR 合到 test 分支

---

## 11. 关联文档

- Plan：`/workspace/docs/superpowers/plans/2026-06-16-color-card-plan.md`
- API 文档：`/workspace/docs/color-card-api.md`
- 用户手册：`/workspace/docs/color-card-user-manual.md`
- 部署指南：`/workspace/docs/color-card-deployment-guide.md`
- 关联 spec：
  - P0-1 销售报价单：`docs/superpowers/specs/2026-06-16-sales-quotation-design.md`
  - P0-2 主备隔离：`docs/superpowers/specs/2026-06-16-failover-isolation-design.md`
  - P0-3 定制订单：`docs/superpowers/specs/2026-06-16-custom-order-design.md`

---

> 文档版本：v1.0 | 2026-06-17 | P0-4 色卡仓储管理模块
