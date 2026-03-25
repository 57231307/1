# 秉羲 ERP 功能模块集成 - 快速参考卡

## 📋 一、文档索引

| 文档 | 路径 | 用途 |
|------|------|------|
| 📄 集成规划 | `docs/integration-plan.md` | 总体规划和目录结构 |
| 📊 数据库设计 | `docs/database-extension.md` | 完整的数据库表设计 |
| 🚀 实施指南 | `docs/IMPLEMENTATION.md` | 详细实施步骤 |
| ⚡ 快速参考 | `docs/QUICK_REFERENCE.md` | 本文档 |

---

## 🎯 二、功能模块总览

### 7 大新增模块

```
┌─────────────────────────────────────────────────────┐
│  模块 1: OA 协同办公                                  │
│  - 通知公告 | 车辆管理 | 会议室 | 印章管理           │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  模块 2: HRM 人力资源                                 │
│  - 员工档案 | 招聘 | 入职 | 考勤 | 薪酬 | 绩效      │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  模块 3: BPM 流程引擎                                 │
│  - 流程设计器 | 审批流 | 任务管理 | 工作流          │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  模块 4: CRM 扩展                                     │
│  - 线索管理 | 商机管理 | 联系人                      │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  模块 5: 日志管理                                     │
│  - 操作日志 | 登录日志 | API 日志 | 链路追踪         │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  模块 6: 数据可视化                                   │
│  - 报表设计器 | 大屏设计器 | 图表管理                │
└─────────────────────────────────────────────────────┘
```

---

## 🗓️ 三、实施时间表

```
阶段一 (1-2 月): 核心基础
├─ 周 1:    项目准备
├─ 周 2-3:  日志管理 ✅
├─ 周 4-5:  通知公告 ✅
└─ 周 6-8:  BPM 流程引擎 ✅

阶段二 (3-4 月): 人力资源 + CRM
├─ 周 9-11: HRM 员工档案 ✅
├─ 周 12-14: HRM 考勤薪酬 ✅
└─ 周 15-16: CRM 扩展 ✅

阶段三 (5-6 月): 可视化 + 售后
├─ 周 17-19: 数据可视化 ✅
├─ 周 20-21: 商城售后 ✅
└─ 周 22-24: 集成测试 ✅
```

---

## 📁 四、目录结构速查

### 后端目录
```
backend/src/
├── handlers/
│   ├── oa/           # OA 模块
│   ├── hrm/          # HRM 模块
│   ├── bpm/          # BPM 模块
│   ├── crm/          # CRM 模块
│   ├── mall/         # 商城模块
│   ├── infra/        # 基础设施
│   └── report/       # 报表模块
├── services/         # 服务层 (同上)
├── models/           # 数据模型 (同上)
└── routes/           # 路由配置
```

### 前端目录
```
frontend/src/
├── components/
│   ├── oa/
│   ├── hrm/
│   ├── bpm/
│   ├── crm/
│   ├── mall/
│   └── report/
├── pages/            # 页面 (同上)
└── services/         # API 服务
```

---

## 🗄️ 五、核心数据库表

### OA 模块 (8 张表)
```sql
oa_notice              -- 通知公告
oa_notice_record       -- 通知阅读记录
oa_vehicle             -- 车辆管理
oa_vehicle_application -- 用车申请
oa_meeting_room        -- 会议室
oa_meeting_reservation -- 会议室预订
oa_seal                -- 印章管理
oa_seal_application    -- 用印申请
```

### HRM 模块 (15 张表)
```sql
hrm_employee           -- 员工档案
hrm_employee_education -- 教育经历
hrm_employee_work_experience -- 工作经历
hrm_employee_family    -- 家庭成员
hrm_onboarding         -- 入职申请
hrm_attendance         -- 考勤记录
hrm_leave_application  -- 请假申请
hrm_salary             -- 薪酬表
-- ... 更多见数据库设计文档
```

### BPM 模块 (12 张表)
```sql
bpm_process_definition -- 流程定义
bpm_process_instance   -- 流程实例
bpm_task               -- 流程任务
bpm_operation_log      -- 操作日志
-- ... 更多见数据库设计文档
```

---

## 🔧 六、常用命令

### 数据库迁移
```bash
# 运行迁移
sea-orm-cli migrate up

# 回滚迁移
sea-orm-cli migrate down

# 生成模型
sea-orm-cli generate entity -o src/models/oa
```

### 开发运行
```bash
# 后端
cd backend
cargo run

# 前端
cd frontend
trunk serve
```

### 生产构建
```bash
# 后端
cd backend
cargo build --release

# 前端
cd frontend
trunk build --release
```

### 代码检查
```bash
# 格式化
cargo fmt

# Clippy 检查
cargo clippy

# 测试
cargo test
```

---

## 🎯 七、API 路由规范

### 路由前缀
```
/api/v1/erp/{module}/{resource}
```

### 示例路由
```
# OA 模块
GET    /api/v1/erp/oa/notices          # 获取通知列表
POST   /api/v1/erp/oa/notices          # 创建通知
GET    /api/v1/erp/oa/notices/:id      # 获取通知详情

# HRM 模块
GET    /api/v1/erp/hrm/employees       # 获取员工列表
POST   /api/v1/erp/hrm/employees       # 创建员工
GET    /api/v1/erp/hrm/employees/:id   # 获取员工详情

# BPM 模块
GET    /api/v1/erp/bpm/definitions     # 获取流程定义
POST   /api/v1/erp/bpm/definitions     # 创建流程定义
POST   /api/v1/erp/bpm/instances       # 发起流程
```

---

## 📊 八、开发优先级

### P0 - 必须实现
- ✅ 日志管理
- ✅ 通知公告
- ✅ BPM 基础流程

### P1 - 重要功能
- ✅ HRM 员工档案
- ✅ HRM 考勤薪酬
- ✅ CRM 扩展

### P2 - 增强功能
- ✅ 数据可视化

---

## 🧪 九、测试要点

### 单元测试
```rust
#[test]
fn test_create_notice() {
    // 测试通知创建
}

#[test]
fn test_employee_calc_salary() {
    // 测试薪酬计算
}
```

### 集成测试
```rust
#[tokio::test]
async fn test_notice_api() {
    // 测试 API 端点
}
```

---

## 📝 十、Git 提交模板

```bash
# 新功能
feat(oa): 添加通知公告功能

# Bug 修复
fix(hrm): 修复考勤统计错误

# 文档
docs: 更新数据库设计文档

# 重构
refactor(bpm): 优化流程引擎代码

# 测试
test: 添加通知模块单元测试
```

---

## ⚠️ 十一、常见陷阱

### 1. 数据库设计
- ❌ 忘记添加索引
- ❌ 缺少外键约束
- ❌ 没有中文注释

### 2. 代码规范
- ❌ 命名不统一
- ❌ 缺少错误处理
- ❌ 没有日志记录

### 3. 性能问题
- ❌ N+1 查询问题
- ❌ 没有分页
- ❌ 缺少缓存

---

## 🎓 十二、学习路径

### 第 1 周
- ✅ 学习 Axum 框架
- ✅ 学习 SeaORM
- ✅ 熟悉项目结构

### 第 2 周
- ✅ 学习 Yew 前端
- ✅ 练习 Rust 异步编程
- ✅ 完成第一个模块

### 第 3-4 周
- ✅ 深入理解 BPM 引擎
- ✅ 学习工作流设计
- ✅ 实现复杂业务逻辑

---

## 📞 十三、获取帮助

### 遇到问题时:
1. ✅ 查看文档：`docs/` 目录
2. ✅ 搜索代码：使用 IDE 搜索功能
3. ✅ 查看示例：参考现有模块代码
4. ✅ 提问：在项目 Issue 中提问

---

## 🎉 成功标准

### 功能完成
- ✅ 所有功能已实现
- ✅ 测试全部通过
- ✅ 文档完整

### 质量达标
- ✅ 代码规范
- ✅ 性能良好
- ✅ 无严重 Bug

### 用户满意
- ✅ 易用性良好
- ✅ 功能实用
- ✅ 反馈积极

---

## 🚀 开始实施

### 第一步：阅读文档
```bash
# 阅读集成规划
cat docs/integration-plan.md

# 阅读数据库设计
cat docs/database-extension.md

# 阅读实施指南
cat docs/IMPLEMENTATION.md
```

### 第二步：环境准备
```bash
# 克隆项目
git clone <repo-url>

# 创建分支
git checkout -b feature/new-modules

# 安装依赖
cd backend && cargo build
```

### 第三步：开始开发
```bash
# 从简单的模块开始
# 例如：日志管理模块

# 1. 创建数据库表
# 2. 生成 SeaORM 模型
# 3. 实现 Service 层
# 4. 实现 Handler 层
# 5. 创建路由
# 6. 前端开发
# 7. 测试
```

---

## 📌 重要提醒

1. ✅ **严格按照阶段实施** - 不要跳阶段
2. ✅ **保证代码质量** - 使用 rustfmt 和 clippy
3. ✅ **充分测试** - 单元测试 + 集成测试
4. ✅ **文档完整** - 代码注释 + API 文档
5. ✅ **持续优化** - 性能优化 + 代码重构

---

## 🎯 下一步行动

### 立即行动:
1. [ ] 阅读所有文档
2. [ ] 配置开发环境
3. [ ] 创建 Git 分支
4. [ ] 从 P0 模块开始开发

### 本周目标:
- [ ] 完成项目准备
- [ ] 实现日志管理模块
- [ ] 开始通知公告模块

### 本月目标:
- [ ] 完成阶段一所有功能
- [ ] 通过所有测试
- [ ] 准备进入阶段二

---

**祝您开发顺利！有任何问题随时查阅文档或提问！** 🎉
