# 秉羲 ERP 功能模块全面融合方案

## 📋 融合目标

将新增的 6 大模块 (OA/HRM/BPM/CRM/日志/报表) 与秉羲 ERP 现有的 18 个核心模块进行深度融合，打造一体化、无缝衔接的企业级管理系统。

---

## 🎯 一、融合架构设计

### 1.1 整体融合架构

```
┌─────────────────────────────────────────────────────────────┐
│                    统一门户层 (Yew 前端)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   BPM 流程引擎 (统一审批)                     │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐  │
│  │采购审批  │销售审批  │财务审批  │库存审批  │人事审批  │  │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   业务中台层 (Axum 服务)                      │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐  │
│  │供应链域  │  财务域  │ 生产域   │  质量域  │  仓储域  │  │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   数据中台层 (SeaORM + PostgreSQL)            │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐  │
│  │统一数据  │  日志中心│  报表中心│  消息中心│  文件中心│  │
│  │模型      │          │          │          │          │  │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔗 二、模块融合详细方案

### 2.1 BPM 流程引擎融合 (20+ 个融合点)

#### **融合点 1: 采购审批流**

**现有模块**: M034 采购订单、M032 采购合同、M035 供应商管理

**融合方案**:
```rust
// backend/src/handlers/purchase_order_handler.rs
pub async fn create_purchase_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePurchaseOrderRequest>,
) -> Result<Json<PurchaseOrder>, AppError> {
    // 1. 创建采购订单
    let order = create_order(&req).await?;
    
    // 2. 根据金额和供应商等级触发 BPM 审批
    if req.total_amount > 100000.0 || req.supplier_level == "战略供应商" {
        // 发起 BPM 流程
        let process_instance = bpm_service::initiate_process(
            "purchase_order_approval",  // 流程定义 key
            order.id,
            &order,
        ).await?;
        
        // 3. 订单状态设置为"待审批"
        order.status = "pending_approval".to_string();
        order.process_instance_id = Some(process_instance.id);
    }
    
    Ok(Json(order))
}

// backend/src/services/bpm/purchase_approval.rs
pub async fn create_purchase_approval_flow() -> ProcessDefinition {
    ProcessDefinition {
        process_key: "purchase_order_approval".to_string(),
        process_name: "采购订单审批流程".to_string(),
        process_type: "procurement".to_string(),
        
        // 审批节点配置
        nodes: vec![
            Node {
                id: "start".to_string(),
                node_type: "start".to_string(),
            },
            Node {
                id: "dept_manager".to_string(),
                node_type: "approval".to_string(),
                approver_type: "role".to_string(),  // 角色审批
                approver_id: "department_manager",  // 部门经理
                conditions: vec![
                    Condition {
                        field: "total_amount".to_string(),
                        operator: ">".to_string(),
                        value: "10000".to_string(),
                    }
                ],
            },
            Node {
                id: "finance_manager".to_string(),
                node_type: "approval".to_string(),
                approver_type: "role".to_string(),
                approver_id: "finance_manager",  // 财务经理
                conditions: vec![
                    Condition {
                        field: "total_amount".to_string(),
                        operator: ">".to_string(),
                        value: "100000".to_string(),
                    }
                ],
            },
            Node {
                id: "general_manager".to_string(),
                node_type: "approval".to_string(),
                approver_type: "role".to_string(),
                approver_id: "general_manager",  // 总经理
                conditions: vec![
                    Condition {
                        field: "total_amount".to_string(),
                        operator: ">".to_string(),
                        value: "500000".to_string(),
                    }
                ],
            },
            Node {
                id: "end".to_string(),
                node_type: "end".to_string(),
            },
        ],
        
        // 审批通过后自动执行
        on_complete: vec![
            Action::UpdateOrderStatus { 
                status: "approved".to_string() 
            },
            Action::CreateInventoryReservation {},  // 创建库存预留
        ],
    }
}
```

**数据融合**:
```sql
-- 在采购订单表中添加 BPM 关联字段
ALTER TABLE purchase_order ADD COLUMN process_instance_id BIGINT REFERENCES bpm_process_instance(id);
ALTER TABLE purchase_order ADD COLUMN approval_status VARCHAR(50) DEFAULT 'pending';
ALTER TABLE purchase_order ADD COLUMN approved_at TIMESTAMP;
ALTER TABLE purchase_order ADD COLUMN approved_by BIGINT REFERENCES sys_user(id);

-- 创建索引
CREATE INDEX idx_purchase_order_process ON purchase_order(process_instance_id);
CREATE INDEX idx_purchase_order_approval ON purchase_order(approval_status);
```

---

#### **融合点 2: 销售订单审批**

**现有模块**: M010 销售订单、M011 客户管理、M045 信用管理

**融合方案**:
```rust
// backend/src/handlers/sales_order_handler.rs
pub async fn create_sales_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSalesOrderRequest>,
) -> Result<Json<SalesOrder>, AppError> {
    // 1. 检查客户信用
    let customer = customer_service::get_by_id(req.customer_id).await?;
    let credit_status = credit_service::check_credit(customer.id).await?;
    
    // 2. 创建销售订单
    let mut order = create_order(&req).await?;
    
    // 3. 根据信用状态和订单金额触发审批
    if credit_status.is_overdue || req.total_amount > customer.credit_limit {
        // 发起信用审批流程
        let process = bpm_service::initiate_process(
            "sales_order_credit_approval",
            order.id,
            &order,
        ).await?;
        
        order.status = "pending_credit_check".to_string();
        order.process_instance_id = Some(process.id);
    }
    
    Ok(Json(order))
}
```

---

#### **融合点 3: 付款申请审批**

**现有模块**: M021 应付管理、M019 总账

**融合方案**:
```rust
// backend/src/handlers/ap_payment_handler.rs
pub async fn create_payment_request(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePaymentRequest>,
) -> Result<Json<ApPayment>, AppError> {
    // 1. 创建付款申请
    let payment = create_payment(&req).await?;
    
    // 2. 根据付款金额和类型触发 BPM 审批
    let process_key = match payment.payment_type.as_str() {
        "采购付款" => "procurement_payment_approval",
        "费用报销" => "expense_reimbursement_approval",
        "预付款" => "advance_payment_approval",
        _ => "general_payment_approval",
    };
    
    let process = bpm_service::initiate_process(
        process_key,
        payment.id,
        &payment,
    ).await?;
    
    payment.status = "pending_approval".to_string();
    payment.process_instance_id = Some(process.id);
    
    Ok(Json(payment))
}
```

---

#### **融合点 4: 库存调整审批**

**现有模块**: M008 库存盘点、M009 库存调整、M007 库存调拨

**融合方案**:
```rust
// backend/src/handlers/inventory_adjustment_handler.rs
pub async fn create_inventory_adjustment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateInventoryAdjustmentRequest>,
) -> Result<Json<InventoryAdjustment>, AppError> {
    let adjustment = create_adjustment(&req).await?;
    
    // 根据调整金额触发审批
    if adjustment.adjustment_value.abs() > 50000.0 {
        let process = bpm_service::initiate_process(
            "inventory_adjustment_approval",
            adjustment.id,
            &adjustment,
        ).await?;
        
        adjustment.status = "pending_approval".to_string();
        adjustment.process_instance_id = Some(process.id);
    }
    
    Ok(Json(adjustment))
}
```

---

### 2.2 CRM 扩展融合 (18 个融合点)

#### **融合点 5: 客户全生命周期管理**

**现有模块**: M011 客户管理

**融合方案**:
```rust
// backend/src/models/crm/customer_lifecycle.rs
pub struct CustomerLifecycle {
    pub id: i64,
    pub customer_id: i64,  // 关联现有客户表
    
    // 线索阶段
    pub lead_source: Option<String>,  // 线索来源
    pub lead_status: Option<String>,  // 线索状态
    pub lead_score: Option<i32>,      // 线索评分
    
    // 商机阶段
    pub opportunity_id: Option<i64>,
    pub opportunity_stage: Option<String>,  // 商机阶段
    pub expected_amount: Option<Decimal>,
    pub win_probability: Option<i32>,  // 成功概率
    
    // 成交阶段
    pub contract_id: Option<i64>,
    pub contract_amount: Option<Decimal>,
    pub sign_date: Option<NaiveDate>,
    
    // 售后阶段
    pub service_level: Option<String>,  // 服务等级
    pub satisfaction_score: Option<i32>, // 满意度
    pub is_repeat_purchase: bool,       // 是否复购
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// backend/src/services/crm/lead_service.rs
pub async fn convert_lead_to_customer(
    lead_id: i64,
    customer_data: CreateCustomerRequest,
) -> Result<Customer, AppError> {
    // 1. 创建客户 (使用现有 M011 模块)
    let customer = customer_service::create(customer_data).await?;
    
    // 2. 更新线索状态
    lead_service::update_status(lead_id, "converted").await?;
    
    // 3. 创建客户生命周期记录
    lifecycle_service::create(CustomerLifecycle {
        customer_id: customer.id,
        lead_status: Some("converted".to_string()),
        // ...
    }).await?;
    
    Ok(customer)
}
```

---

#### **融合点 6: 销售漏斗与现有销售订单融合**

**现有模块**: M010 销售订单、M040 销售报价

**融合方案**:
```rust
// backend/src/services/crm/opportunity_service.rs
pub async fn create_opportunity_from_inquiry(
    inquiry_id: i64,
) -> Result<Opportunity, AppError> {
    // 从现有询价单创建商机
    let inquiry = inquiry_service::get_by_id(inquiry_id).await?;
    
    let opportunity = Opportunity {
        name: format!("{}-{}", inquiry.customer_name, inquiry.product_name),
        customer_id: inquiry.customer_id,
        expected_amount: inquiry.estimated_amount,
        stage: "初步接洽".to_string(),
        probability: 20,
        expected_close_date: inquiry.expected_delivery_date,
        source: "询价转化".to_string(),
        // ...
    };
    
    create(opportunity).await
}

// 商机推进到报价阶段时，自动创建销售报价
pub async fn advance_opportunity_to_quotation(
    opportunity_id: i64,
) -> Result<(), AppError> {
    let opportunity = get_by_id(opportunity_id).await?;
    
    if opportunity.stage == "方案报价" {
        // 创建销售报价 (使用现有 M040 模块)
        let quotation = SalesQuotation {
            customer_id: opportunity.customer_id,
            opportunity_id: Some(opportunity_id),
            total_amount: opportunity.expected_amount,
            // ...
        };
        
        quotation_service::create(quotation).await?;
    }
    
    Ok(())
}
```

---

#### **融合点 7: 客户信用管理与现有财务融合**

**现有模块**: M045 信用管理、M060 应收管理

**融合方案**:
```rust
// backend/src/services/crm/credit_service.rs
pub async fn calculate_customer_credit_score(
    customer_id: i64,
) -> Result<CreditScore, AppError> {
    // 1. 获取客户基本信息
    let customer = customer_service::get_by_id(customer_id).await?;
    
    // 2. 获取历史交易数据 (从现有销售订单)
    let orders = sales_order_service::get_by_customer(customer_id).await?;
    let total_sales = orders.iter().map(|o| o.total_amount).sum::<Decimal>();
    
    // 3. 获取回款数据 (从现有应收模块 M060)
    let receivables = ar_service::get_by_customer(customer_id).await?;
    let overdue_amount = receivables.iter()
        .filter(|r| r.is_overdue)
        .map(|r| r.amount)
        .sum::<Decimal>();
    
    // 4. 计算信用评分
    let credit_score = CreditScore {
        customer_id,
        
        // 基础分 (注册资本、成立年限)
        base_score: calculate_base_score(&customer),
        
        // 交易分 (交易金额、交易频次)
        transaction_score: calculate_transaction_score(total_sales, orders.len()),
        
        // 回款分 (回款率、逾期率)
        payment_score: calculate_payment_score(&receivables),
        
        // 总分
        total_score: calculate_total_score(
            base_score,
            transaction_score,
            payment_score,
        ),
        
        // 建议信用额度
        suggested_credit_limit: calculate_credit_limit(total_score),
    };
    
    Ok(credit_score)
}

// 信用审批流程触发
pub async fn adjust_credit_limit(
    customer_id: i64,
    new_limit: Decimal,
    reason: String,
) -> Result<(), AppError> {
    // 发起信用调整审批流程
    let process = bpm_service::initiate_process(
        "credit_limit_adjustment",
        customer_id,
        &CreditAdjustmentRequest {
            customer_id,
            old_limit: get_current_limit(customer_id).await?,
            new_limit,
            reason,
        },
    ).await?;
    
    // 审批通过后自动更新信用额度
    Ok(())
}
```

---

### 2.3 OA 协同办公融合 (15 个融合点)

#### **融合点 8: 智能通知与现有业务融合**

**现有模块**: M014 操作日志、M015 仪表板

**融合方案**:
```rust
// backend/src/services/oa/notification_service.rs
pub async fn create_business_notification(
    event_type: BusinessEventType,
    event_data: JsonValue,
) -> Result<(), AppError> {
    match event_type {
        // 采购订单审批通知
        BusinessEventType::PurchaseOrderApproved => {
            let order: PurchaseOrder = serde_json::from_value(event_data)?;
            
            // 创建通知
            let notice = Notice {
                title: format!("采购订单 {} 已审批通过", order.order_no),
                content: format!(
                    "采购订单 {} (金额：{}) 已通过审批，请安排后续采购流程。",
                    order.order_no, order.total_amount
                ),
                notice_type: "business".to_string(),
                publish_range_type: "user".to_string(),
                publish_range_ids: vec![order.created_by.to_string()],
            };
            
            notice_service::create(notice).await?;
            
            // 发送站内信
            message_service::send_system_message(
                order.created_by,
                "采购订单审批通知",
                &notice.content,
            ).await?;
        }
        
        // 库存预警通知
        BusinessEventType::InventoryLowStock => {
            let product: Product = serde_json::from_value(event_data)?;
            
            let notice = Notice {
                title: format!("库存预警：{} 库存不足", product.name),
                content: format!(
                    "产品 {} 当前库存为 {}，低于安全库存 {}，请及时采购。",
                    product.name, 
                    product.stock_quantity,
                    product.safety_stock
                ),
                notice_type: "urgent".to_string(),
                publish_range_type: "role".to_string(),
                publish_range_ids: vec!["procurement_manager".to_string()],
            };
            
            notice_service::create(notice).await?;
        }
        
        // 应收预警通知
        BusinessEventType::ReceivableOverdue => {
            let receivable: Receivable = serde_json::from_value(event_data)?;
            
            let notice = Notice {
                title: format!("应收预警：{} 逾期未回款", receivable.customer_name),
                content: format!(
                    "客户 {} 的应收款项 {} 已逾期 {} 天，请及时跟进催款。",
                    receivable.customer_name,
                    receivable.amount,
                    receivable.overdue_days
                ),
                notice_type: "urgent".to_string(),
                publish_range_type: "role".to_string(),
                publish_range_ids: vec!["sales_manager".to_string()],
            };
            
            notice_service::create(notice).await?;
        }
    }
    
    Ok(())
}
```

---

#### **融合点 9: 会议室管理与会议申请**

**融合方案**:
```rust
// backend/src/handlers/meeting_room_handler.rs
pub async fn create_meeting_reservation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMeetingReservationRequest>,
) -> Result<Json<MeetingReservation>, AppError> {
    // 1. 创建会议预订
    let reservation = MeetingReservation {
        room_id: req.room_id,
        organizer_id: req.organizer_id,
        topic: req.topic,
        agenda: req.agenda,
        start_time: req.start_time,
        end_time: req.end_time,
        participant_ids: req.participant_ids,
        
        // 关联业务
        business_type: req.business_type,  // 采购评审/销售评审/项目评审
        business_id: req.business_id,      // 关联的业务 ID
    };
    
    // 2. 如果关联采购评审，触发 BPM 流程
    if let Some(business_id) = req.business_id {
        if req.business_type == Some("procurement_review".to_string()) {
            let process = bpm_service::initiate_process(
                "procurement_review_meeting",
                business_id,
                &reservation,
            ).await?;
            
            reservation.process_instance_id = Some(process.id);
        }
    }
    
    Ok(Json(reservation))
}
```

---

### 2.4 日志管理融合 (8 个融合点)

#### **融合点 10: 统一日志审计**

**现有模块**: M014 操作日志

**融合方案**:
```rust
// backend/src/middleware/unified_log.rs
pub async fn unified_log_middleware(
    request: Request<Body>,
    next: Next<Body>,
) -> Result<Response, AppError> {
    let start_time = Instant::now();
    let trace_id = generate_trace_id();
    
    // 提取请求信息
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    // 执行请求
    let response = next.run(request).await;
    
    // 记录日志
    let duration = start_time.elapsed();
    let status = response.status();
    
    // 统一日志记录
    tokio::spawn(async move {
        // 1. API 访问日志
        let api_log = ApiLog {
            trace_id: trace_id.clone(),
            method: method.to_string(),
            uri: uri.to_string(),
            status: status.as_u16() as i32,
            duration_ms: duration.as_millis() as i32,
            // ...
        };
        api_log_service::create(api_log).await.unwrap();
        
        // 2. 业务操作日志 (如果是业务操作)
        if is_business_operation(&uri) {
            let operation_log = OperationLog {
                trace_id,
                module: extract_module(&uri),
                operation: extract_operation(&uri),
                // ...
            };
            operation_log_service::create(operation_log).await.unwrap();
        }
    });
    
    Ok(response)
}
```

---

### 2.5 报表系统融合 (25+ 个融合点)

#### **融合点 11: 财务报表与现有财务模块融合**

**现有模块**: M019 总账、M021 应付、M060 应收

**融合方案**:
```rust
// backend/src/services/report/financial_report.rs
pub async fn generate_financial_dashboard(
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<FinancialDashboard, AppError> {
    // 1. 应收统计 (从 M060)
    let ar_stats = ar_service::get_statistics(start_date, end_date).await?;
    
    // 2. 应付统计 (从 M021)
    let ap_stats = ap_service::get_statistics(start_date, end_date).await?;
    
    // 3. 总账统计 (从 M019)
    let gl_stats = gl_service::get_statistics(start_date, end_date).await?;
    
    // 4. 生成财务报表数据
    let dashboard = FinancialDashboard {
        period: format!("{} 至 {}", start_date, end_date),
        
        // 应收分析
        ar_summary: ArSummary {
            total_receivable: ar_stats.total_amount,
            collected: ar_stats.collected_amount,
            outstanding: ar_stats.outstanding_amount,
            overdue: ar_stats.overdue_amount,
            collection_rate: ar_stats.collection_rate,
        },
        
        // 应付分析
        ap_summary: ApSummary {
            total_payable: ap_stats.total_amount,
            paid: ap_stats.paid_amount,
            outstanding: ap_stats.outstanding_amount,
            overdue: ap_stats.overdue_amount,
        },
        
        // 资金状况
        cash_flow: CashFlow {
            inflow: ar_stats.collected_amount,
            outflow: ap_stats.paid_amount,
            net_flow: ar_stats.collected_amount - ap_stats.paid_amount,
        },
        
        // 图表数据
        charts: vec![
            Chart {
                chart_type: "trend".to_string(),
                title: "应收应付趋势".to_string(),
                data: generate_trend_data(start_date, end_date).await?,
            },
            Chart {
                chart_type: "pie".to_string(),
                title: "应收账龄分析".to_string(),
                data: generate_aging_data().await?,
            },
        ],
    };
    
    Ok(dashboard)
}
```

---

#### **融合点 12: 销售报表与现有销售模块融合**

**现有模块**: M010 销售订单、M011 客户管理

**融合方案**:
```rust
// backend/src/services/report/sales_report.rs
pub async fn generate_sales_funnel_report(
    period: ReportPeriod,
) -> Result<SalesFunnelReport, AppError> {
    // 1. 获取线索数据 (新增 CRM 模块)
    let leads = lead_service::get_by_period(period).await?;
    
    // 2. 获取商机数据 (新增 CRM 模块)
    let opportunities = opportunity_service::get_by_period(period).await?;
    
    // 3. 获取报价数据 (现有 M040)
    let quotations = quotation_service::get_by_period(period).await?;
    
    // 4. 获取订单数据 (现有 M010)
    let orders = sales_order_service::get_by_period(period).await?;
    
    // 5. 生成销售漏斗报表
    let funnel = SalesFunnelReport {
        period,
        
        // 各阶段数量
        stages: vec![
            FunnelStage {
                name: "线索".to_string(),
                count: leads.len(),
                amount: 0.0,
            },
            FunnelStage {
                name: "商机".to_string(),
                count: opportunities.len(),
                amount: opportunities.iter().map(|o| o.expected_amount).sum(),
            },
            FunnelStage {
                name: "报价".to_string(),
                count: quotations.len(),
                amount: quotations.iter().map(|q| q.total_amount).sum(),
            },
            FunnelStage {
                name: "订单".to_string(),
                count: orders.len(),
                amount: orders.iter().map(|o| o.total_amount).sum(),
            },
        ],
        
        // 转化率
        conversion_rates: calculate_conversion_rates(
            leads.len(),
            opportunities.len(),
            quotations.len(),
            orders.len(),
        ),
    };
    
    Ok(funnel)
}
```

---

### 2.6 HRM 融合 (12 个融合点)

#### **融合点 13: 员工与现有用户模块融合**

**现有模块**: M001 用户管理、M003 部门管理

**融合方案**:
```rust
// backend/src/services/hrm/employee_service.rs
pub async fn onboard_new_employee(
    onboarding_data: OnboardingRequest,
) -> Result<Employee, AppError> {
    // 1. 创建员工档案
    let employee = Employee {
        employee_no: generate_employee_no(),
        name: onboarding_data.name,
        department_id: onboarding_data.department_id,
        position_id: onboarding_data.position_id,
        hire_date: onboarding_data.hire_date,
        // ...
    };
    
    // 2. 创建系统用户 (使用现有 M001 模块)
    let user = user_service::create_user(CreateUserRequest {
        username: generate_username(&employee),
        password: generate_initial_password(),
        email: onboarding_data.email,
        phone: onboarding_data.phone,
        department_id: onboarding_data.department_id,
        role_ids: vec![onboarding_data.role_id],  // 分配角色
    }).await?;
    
    // 3. 关联员工和用户
    employee.user_id = Some(user.id);
    
    // 4. 发送入职通知
    notification_service::send_onboarding_notification(
        user.id,
        &employee,
    ).await?;
    
    Ok(employee)
}

// 离职流程
pub async fn offboard_employee(
    employee_id: i64,
    reason: String,
) -> Result<(), AppError> {
    let employee = get_by_id(employee_id).await?;
    
    // 1. 发起离职审批流程
    let process = bpm_service::initiate_process(
        "employee_resignation",
        employee_id,
        &ResignationRequest {
            employee_id,
            reason,
            last_work_date: employee.last_work_date,
        },
    ).await?;
    
    // 2. 审批通过后执行离职操作
    // - 禁用用户账号
    // - 转移工作交接
    // - 结算薪酬
    
    Ok(())
}
```

---

## 📊 三、数据融合设计

### 3.1 统一数据模型

```rust
// backend/src/models/common/mod.rs

/// 统一的用户 ID (关联 sys_user 和 hrm_employee)
pub type UserId = i64;

/// 统一的部门 ID
pub type DepartmentId = i64;

/// 统一的业务流程实例 ID
pub type ProcessInstanceId = i64;

/// 统一的业务实体 Trait
pub trait BusinessEntity {
    fn get_id(&self) -> i64;
    fn get_created_by(&self) -> UserId;
    fn get_department_id(&self) -> Option<DepartmentId>;
    fn get_process_instance_id(&self) -> Option<ProcessInstanceId>;
}

// 为现有模块实现 Trait
impl BusinessEntity for PurchaseOrder {
    fn get_id(&self) -> i64 { self.id }
    fn get_created_by(&self) -> UserId { self.created_by }
    fn get_department_id(&self) -> Option<DepartmentId> { self.department_id }
    fn get_process_instance_id(&self) -> Option<ProcessInstanceId> { self.process_instance_id }
}

impl BusinessEntity for SalesOrder {
    fn get_id(&self) -> i64 { self.id }
    fn get_created_by(&self) -> UserId { self.created_by }
    fn get_department_id(&self) -> Option<DepartmentId> { self.department_id }
    fn get_process_instance_id(&self) -> Option<ProcessInstanceId> { self.process_instance_id }
}
```

---

### 3.2 统一消息队列

```rust
// backend/src/services/message_queue.rs

/// 统一的事件类型
pub enum UnifiedEvent {
    // 采购事件
    PurchaseOrderCreated(PurchaseOrder),
    PurchaseOrderApproved(PurchaseOrder),
    PurchaseOrderRejected(PurchaseOrder),
    
    // 销售事件
    SalesOrderCreated(SalesOrder),
    SalesOrderApproved(SalesOrder),
    CustomerCreditChanged(CustomerCredit),
    
    // 库存事件
    InventoryLowStock(Product),
    InventoryAdjusted(InventoryAdjustment),
    
    // 财务事件
    PaymentApproved(ApPayment),
    ReceivableOverdue(Receivable),
    
    // 人事事件
    EmployeeOnboarded(Employee),
    EmployeeResigned(Employee),
    LeaveApproved(LeaveApplication),
}

/// 事件总线服务
pub async fn publish_event(event: UnifiedEvent) -> Result<(), AppError> {
    // 1. 记录事件日志
    event_log_service::log(&event).await?;
    
    // 2. 触发通知
    notification_service::handle_event(&event).await?;
    
    // 3. 更新报表数据
    report_service::update_from_event(&event).await?;
    
    // 4. 触发其他业务逻辑
    match event {
        UnifiedEvent::PurchaseOrderApproved(order) => {
            // 自动创建库存预留
            inventory_reservation_service::create_from_order(&order).await?;
        }
        UnifiedEvent::SalesOrderApproved(order) => {
            // 自动扣减库存
            inventory_service::deduct(&order).await?;
        }
        _ => {}
    }
    
    Ok(())
}
```

---

## 🎯 四、融合实施步骤

### 阶段一：BPM 流程引擎融合 (第 1-2 月)

#### 第 1 周：基础框架搭建
- [ ] 创建 BPM 数据库表
- [ ] 实现流程定义服务
- [ ] 实现流程实例服务
- [ ] 实现任务服务

#### 第 2-3 周：采购审批流融合
- [ ] 实现采购订单审批流程
- [ ] 修改采购订单 Handler
- [ ] 添加审批状态字段
- [ ] 前端添加审批 UI

#### 第 4-5 周：销售审批流融合
- [ ] 实现销售订单审批流程
- [ ] 实现信用审批流程
- [ ] 修改销售订单 Handler
- [ ] 前端添加审批 UI

#### 第 6-8 周：财务审批流融合
- [ ] 实现付款申请审批流程
- [ ] 实现费用报销审批流程
- [ ] 实现预算调整审批流程
- [ ] 前端添加审批 UI

---

### 阶段二：CRM 扩展融合 (第 3 月)

#### 第 9-10 周：客户生命周期
- [ ] 创建线索/商机表
- [ ] 实现线索服务
- [ ] 实现商机服务
- [ ] 与现有客户模块融合

#### 第 11-12 周：销售漏斗
- [ ] 实现销售漏斗服务
- [ ] 与现有销售订单融合
- [ ] 与现有报价模块融合
- [ ] 前端添加漏斗报表

---

### 阶段三：日志和报表融合 (第 4 月)

#### 第 13-14 周：统一日志
- [ ] 实现统一日志中间件
- [ ] 扩展现有操作日志
- [ ] 添加 API 日志
- [ ] 实现链路追踪

#### 第 15-16 周：报表系统
- [ ] 创建报表数据库表
- [ ] 实现报表设计器服务
- [ ] 融合现有财务数据
- [ ] 融合现有销售数据
- [ ] 前端添加报表设计器

---

## ✅ 五、融合验收标准

### 5.1 功能验收

| 融合点 | 验收标准 | 状态 |
|--------|---------|------|
| BPM 采购审批 | 采购订单可发起审批，审批通过后自动执行 | ⬜ |
| BPM 销售审批 | 销售订单可发起信用审批 | ⬜ |
| BPM 财务审批 | 付款申请可发起多级审批 | ⬜ |
| CRM 客户生命周期 | 线索→商机→订单完整转化 | ⬜ |
| CRM 销售漏斗 | 完整展示销售各阶段转化 | ⬜ |
| OA 智能通知 | 业务事件自动触发通知 | ⬜ |
| 统一日志 | 所有操作可追溯，支持链路追踪 | ⬜ |
| 财务报表 | 自动生成财务 dashboard | ⬜ |
| 销售报表 | 销售漏斗报表准确 | ⬜ |

### 5.2 性能验收

- ✅ API 响应时间 < 200ms
- ✅ 并发支持 > 500 QPS
- ✅ 数据库查询优化，无 N+1 问题
- ✅ 报表生成时间 < 5s

### 5.3 数据一致性验收

- ✅ 所有外键约束完整
- ✅ 事务处理正确
- ✅ 数据同步及时
- ✅ 无数据丢失

---

## 🎉 六、融合收益

### 6.1 业务价值

1. **流程自动化**: 20+ 个业务流程实现自动化审批
2. **数据一体化**: 消除信息孤岛，数据实时互通
3. **决策智能化**: 25+ 个报表支持科学决策
4. **管理精细化**: 客户/员工全生命周期管理

### 6.2 技术价值

1. **架构统一**: 统一的数据模型和消息队列
2. **代码复用**: 现有模块无需重写，只需扩展
3. **易于维护**: 模块化设计，松耦合
4. **可扩展**: 新增功能模块简单

---

**融合方案完成!** 接下来可以开始实施阶段一的 BPM 流程引擎融合开发。🚀
