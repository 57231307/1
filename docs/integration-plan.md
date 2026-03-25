# 秉羲 ERP 功能模块集成规划

## 📋 集成目标

将 Deep Office 的核心功能模块融入秉羲 ERP 项目，增强系统的企业级应用能力，同时保持面料行业特色。

## 🗂️ 一、新增目录结构

### 1.1 后端目录扩展

```
backend/
├── src/
│   ├── config/
│   │   ├── settings.rs          # 配置管理
│   │   └── modules.rs           # 模块配置 (新增)
│   ├── database/
│   │   ├── connection.rs
│   │   └── migration.rs         # 数据库迁移 (新增)
│   ├── handlers/
│   │   # === 现有模块 ===
│   │   ├── auth_handler.rs
│   │   ├── user_handler.rs
│   │   ├── product_handler.rs
│   │   ├── sales_order_handler.rs
│   │   └── ...
│   │   
│   │   # === 新增 OA 协同办公 ===
│   │   ├── oa/
│   │   │   ├── mod.rs
│   │   │   ├── notice_handler.rs        # 通知公告
│   │   │   ├── vehicle_handler.rs       # 车辆管理
│   │   │   ├── seal_handler.rs          # 印章管理
│   │   │   ├── meeting_room_handler.rs  # 会议室管理
│   │   │   └── cloud_disk_handler.rs    # 企业云盘
│   │   │
│   │   # === 新增 HRM 人力资源 ===
│   │   ├── hrm/
│   │   │   ├── mod.rs
│   │   │   ├── employee_handler.rs      # 员工档案
│   │   │   ├── recruitment_handler.rs   # 招聘管理
│   │   │   ├── onboarding_handler.rs    # 入职管理
│   │   │   ├── regularization_handler.rs # 转正管理
│   │   │   ├── transfer_handler.rs      # 调动管理
│   │   │   ├── resignation_handler.rs   # 离职管理
│   │   │   ├── attendance_handler.rs    # 考勤管理
│   │   │   ├── salary_handler.rs        # 薪酬管理
│   │   │   └── performance_handler.rs   # 绩效管理
│   │   │
│   │   # === 新增 BPM 流程引擎 ===
│   │   ├── bpm/
│   │   │   ├── mod.rs
│   │   │   ├── process_designer_handler.rs  # 流程设计器
│   │   │   ├── process_instance_handler.rs  # 流程实例
│   │   │   ├── task_handler.rs              # 任务管理
│   │   │   ├── approval_handler.rs          # 审批管理
│   │   │   └── workflow_handler.rs          # 工作流
│   │   │
│   │   # === 扩展 CRM 客户管理 ===
│   │   ├── crm/
│   │   │   ├── mod.rs
│   │   │   ├── lead_handler.rs              # 线索管理 (新增)
│   │   │   ├── opportunity_handler.rs       # 商机管理 (新增)
│   │   │   ├── contact_handler.rs           # 联系人 (新增)
│   │   │   ├── contract_handler.rs          # 合同管理 (已有)
│   │   │   └── customer_handler.rs          # 客户管理 (已有)
│   │   │
│   │   # === 新增日志和追踪 ===
│   │   ├── infra/
│   │   │   ├── mod.rs
│   │   │   ├── operation_log_handler.rs     # 操作日志 (已有)
│   │   │   ├── login_log_handler.rs         # 登录日志 (新增)
│   │   │   ├── api_log_handler.rs           # API 日志 (新增)
│   │   │   └── trace_handler.rs             # 链路追踪 (新增)
│   │   │
│   │   # === 新增数据可视化 ===
│   │   ├── report/
│   │   │   ├── mod.rs
│   │   │   ├── dashboard_handler.rs         # 仪表板 (已有)
│   │   │   ├── report_designer_handler.rs   # 报表设计器 (新增)
│   │   │   ├── big_screen_handler.rs        # 大屏设计器 (新增)
│   │   │   └── chart_handler.rs             # 图表管理 (新增)
│   │   │
│   │   └── mod.rs                   # Handler 模块总入口
│   │
│   ├── middleware/
│   │   ├── auth.rs                  # 认证中间件 (已有)
│   │   ├── operation_log.rs         # 操作日志中间件 (已有)
│   │   ├── request_log.rs           # 请求日志中间件 (新增)
│   │   ├── trace_log.rs             # 链路追踪中间件 (新增)
│   │   ├── rate_limit.rs            # 限流中间件 (新增)
│   │   └── mod.rs
│   │
│   ├── models/
│   │   # === 现有模型 ===
│   │   ├── user.rs
│   │   ├── product.rs
│   │   └── ...
│   │   │
│   │   # === 新增 OA 模型 ===
│   │   ├── oa/
│   │   │   ├── mod.rs
│   │   │   ├── notice.rs            # 通知公告
│   │   │   ├── vehicle.rs           # 车辆管理
│   │   │   ├── seal.rs              # 印章管理
│   │   │   ├── meeting_room.rs      # 会议室
│   │   │   └── cloud_disk.rs        # 云盘
│   │   │
│   │   # === 新增 HRM 模型 ===
│   │   ├── hrm/
│   │   │   ├── mod.rs
│   │   │   ├── employee.rs          # 员工档案
│   │   │   ├── recruitment.rs       # 招聘
│   │   │   ├── onboarding.rs        # 入职
│   │   │   ├── attendance.rs        # 考勤
│   │   │   ├── salary.rs            # 薪酬
│   │   │   └── performance.rs       # 绩效
│   │   │
│   │   # === 新增 BPM 模型 ===
│   │   ├── bpm/
│   │   │   ├── mod.rs
│   │   │   ├── process_definition.rs  # 流程定义
│   │   │   ├── process_instance.rs    # 流程实例
│   │   │   ├── task.rs                # 任务
│   │   │   ├── approval_flow.rs       # 审批流
│   │   │   └── workflow.rs            # 工作流
│   │   │
│   │   # === 新增 CRM 模型 ===
│   │   ├── crm/
│   │   │   ├── mod.rs
│   │   │   ├── lead.rs              # 线索
│   │   │   ├── opportunity.rs       # 商机
│   │   │   └── contact.rs           # 联系人
│   │   │
│   │   # === 新增日志模型 ===
│   │   ├── infra/
│   │   │   ├── mod.rs
│   │   │   ├── operation_log.rs     # 操作日志 (已有)
│   │   │   ├── login_log.rs         # 登录日志
│   │   │   └── api_log.rs           # API 日志
│   │   │
│   │   # === 新增报表模型 ===
│   │   ├── report/
│   │   │   ├── mod.rs
│   │   │   ├── report_template.rs   # 报表模板
│   │   │   ├── chart.rs             # 图表
│   │   │   └── dashboard.rs         # 仪表板
│   │   │
│   │   └── mod.rs                   # Model 模块总入口
│   │
│   ├── services/
│   │   # === 现有服务 ===
│   │   ├── user_service.rs
│   │   ├── product_service.rs
│   │   └── ...
│   │   │
│   │   # === 新增 OA 服务 ===
│   │   ├── oa/
│   │   │   ├── mod.rs
│   │   │   ├── notice_service.rs
│   │   │   ├── vehicle_service.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 HRM 服务 ===
│   │   ├── hrm/
│   │   │   ├── mod.rs
│   │   │   ├── employee_service.rs
│   │   │   ├── recruitment_service.rs
│   │   │   ├── attendance_service.rs
│   │   │   ├── salary_service.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 BPM 服务 ===
│   │   ├── bpm/
│   │   │   ├── mod.rs
│   │   │   ├── process_service.rs
│   │   │   ├── workflow_service.rs
│   │   │   ├── task_service.rs
│   │   │   └── approval_service.rs
│   │   │
│   │   # === 新增 CRM 服务 ===
│   │   ├── crm/
│   │   │   ├── mod.rs
│   │   │   ├── lead_service.rs
│   │   │   ├── opportunity_service.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增日志服务 ===
│   │   ├── infra/
│   │   │   ├── mod.rs
│   │   │   ├── log_service.rs
│   │   │   └── trace_service.rs
│   │   │
│   │   # === 新增报表服务 ===
│   │   ├── report/
│   │   │   ├── mod.rs
│   │   │   ├── report_service.rs
│   │   │   └── chart_service.rs
│   │   │
│   │   └── mod.rs                   # Service 模块总入口
│   │
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── common.rs
│   │   ├── jwt.rs
│   │   ├── excel.rs
│   │   ├── pdf.rs                   # PDF 生成 (新增)
│   │   ├── sms.rs                   # 短信发送 (新增)
│   │   ├── email.rs                 # 邮件发送 (已有)
│   │   └── wechat.rs                # 微信推送 (新增)
│   │
│   ├── routes/
│   │   ├── mod.rs                   # 路由总入口
│   │   ├── oa_routes.rs             # OA 路由 (新增)
│   │   ├── hrm_routes.rs            # HRM 路由 (新增)
│   │   ├── bpm_routes.rs            # BPM 路由 (新增)
│   │   ├── crm_routes.rs            # CRM 路由扩展 (新增)
│   │   ├── infra_routes.rs          # 基础设施路由 (新增)
│   │   └── report_routes.rs         # 报表路由 (新增)
│   │
│   ├── grpc/
│   │   ├── mod.rs
│   │   ├── service.rs
│   │   └── proto/
│   │       ├── user.proto
│   │       ├── auth.proto
│   │       ├── bpm.proto            # BPM gRPC (新增)
│   │       └── report.proto         # 报表 gRPC (新增)
│   │
│   ├── lib.rs
│   │   └── main.rs
│   │
│   └── Cargo.toml
│
├── migrations/                      # 数据库迁移脚本 (新增)
│   ├── 001_init.sql
│   ├── 002_add_oa_module.sql
│   ├── 003_add_hrm_module.sql
│   ├── 004_add_bpm_module.sql
│   └── ...
│
├── scripts/
│   ├── dev.sh                       # 开发环境脚本
│   ├── prod.sh                      # 生产环境脚本
│   ├── backup.sh                    # 数据库备份
│   └── migrate.sh                   # 数据库迁移
│
└── docs/
    ├── api-docs.md
    ├── deployment.md
    ├── module-design/
    │   ├── oa-module.md
    │   ├── hrm-module.md
    │   ├── bpm-module.md
    │   └── ...
    └── development-guide.md
```

### 1.2 前端目录扩展

```
frontend/
├── src/
│   ├── app.rs
│   ├── components/
│   │   ├── common/
│   │   │   ├── mod.rs
│   │   │   ├── header.rs
│   │   │   ├── sidebar.rs
│   │   │   ├── footer.rs
│   │   │   ├── breadcrumb.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 OA 组件 ===
│   │   ├── oa/
│   │   │   ├── mod.rs
│   │   │   ├── notice_list.rs
│   │   │   ├── notice_detail.rs
│   │   │   ├── notice_editor.rs
│   │   │   ├── vehicle_calendar.rs
│   │   │   ├── meeting_room_picker.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 HRM 组件 ===
│   │   ├── hrm/
│   │   │   ├── mod.rs
│   │   │   ├── employee_card.rs
│   │   │   ├── employee_table.rs
│   │   │   ├── org_chart.rs
│   │   │   ├── attendance_calendar.rs
│   │   │   ├── salary_slip.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 BPM 组件 ===
│   │   ├── bpm/
│   │   │   ├── mod.rs
│   │   │   ├── process_designer.rs
│   │   │   ├── form_designer.rs
│   │   │   ├── approval_flow.rs
│   │   │   ├── task_list.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 CRM 组件 ===
│   │   ├── crm/
│   │   │   ├── mod.rs
│   │   │   ├── lead_card.rs
│   │   │   ├── opportunity_funnel.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增报表组件 ===
│   │   ├── report/
│   │   │   ├── mod.rs
│   │   │   ├── chart_card.rs
│   │   │   ├── report_grid.rs
│   │   │   ├── big_screen.rs
│   │   │   └── ...
│   │   │
│   │   └── mod.rs
│   │
│   ├── pages/
│   │   ├── mod.rs
│   │   ├── home.rs
│   │   ├── login.rs
│   │   │
│   │   # === 新增 OA 页面 ===
│   │   ├── oa/
│   │   │   ├── mod.rs
│   │   │   ├── notice_list_page.rs
│   │   │   ├── notice_detail_page.rs
│   │   │   ├── vehicle_page.rs
│   │   │   ├── meeting_room_page.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 HRM 页面 ===
│   │   ├── hrm/
│   │   │   ├── mod.rs
│   │   │   ├── employee_list_page.rs
│   │   │   ├── employee_detail_page.rs
│   │   │   ├── org_chart_page.rs
│   │   │   ├── attendance_page.rs
│   │   │   ├── salary_page.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 BPM 页面 ===
│   │   ├── bpm/
│   │   │   ├── mod.rs
│   │   │   ├── process_designer_page.rs
│   │   │   ├── my_tasks_page.rs
│   │   │   ├── process_initiation_page.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增 CRM 页面 ===
│   │   ├── crm/
│   │   │   ├── mod.rs
│   │   │   ├── lead_list_page.rs
│   │   │   ├── opportunity_list_page.rs
│   │   │   └── ...
│   │   │
│   │   # === 新增报表页面 ===
│   │   ├── report/
│   │   │   ├── mod.rs
│   │   │   ├── report_designer_page.rs
│   │   │   ├── big_screen_page.rs
│   │   │   └── ...
│   │   │
│   │   └── ...
│   │
│   ├── services/
│   │   ├── mod.rs
│   │   ├── api.rs
│   │   │
│   │   # === 新增 OA 服务 ===
│   │   ├── oa_service.rs
│   │   │
│   │   # === 新增 HRM 服务 ===
│   │   ├── hrm_service.rs
│   │   │
│   │   # === 新增 BPM 服务 ===
│   │   ├── bpm_service.rs
│   │   │
│   │   # === 新增 CRM 服务 ===
│   │   ├── crm_service.rs
│   │   │
│   │   # === 新增报表服务 ===
│   │   └── report_service.rs
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── user.rs
│   │   │
│   │   # === 新增 OA 模型 ===
│   │   ├── oa/
│   │   │   ├── mod.rs
│   │   │   └── notice.rs
│   │   │
│   │   # === 新增 HRM 模型 ===
│   │   ├── hrm/
│   │   │   ├── mod.rs
│   │   │   └── employee.rs
│   │   │
│   │   # === 新增 BPM 模型 ===
│   │   ├── bpm/
│   │   │   ├── mod.rs
│   │   │   └── process.rs
│   │   │
│   │   # === 新增 CRM 模型 ===
│   │   ├── crm/
│   │   │   ├── mod.rs
│   │   │   └── lead.rs
│   │   │
│   │   # === 新增报表模型 ===
│   │   └── report/
│   │       ├── mod.rs
│   │       └── chart.rs
│   │
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── format.rs
│   │   ├── validate.rs
│   │   └── storage.rs
│   │
│   ├── router.rs                    # 前端路由
│   └── main.rs
│
├── styles/
│   ├── mod.css
│   ├── variables.css
│   ├── components/
│   │   ├── oa.css
│   │   ├── hrm.css
│   │   ├── bpm.css
│   │   └── report.css
│   └── ...
│
├── index.html
├── Cargo.toml
└── Trunk.toml
```
