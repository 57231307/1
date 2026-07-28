//! 业务事件 Kafka 线格式序列化（与 BusinessEvent 字段一一对应）
//!
//! 拆分自 event_kafka.rs：原 pub mod payload_serde { ... } 内部块。
//! 包含 EventPayload 枚举 + From<&BusinessEvent> + TryFrom<EventPayload> 三段实现。

/// 为 `BusinessEvent` 增加 `Serialize` / `Deserialize` 派生（仅在 kafka 模块内使用）
/// 原 `BusinessEvent` 派生来自 `event_bus.rs`，没有 `Serialize`。这里通过新类型；`EventPayload` 包装，再借助 `serde_json` 透明转换，避免在 8 个必需文件之外；改动 `event_bus.rs` 的公共定义。
pub mod payload_serde {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    use crate::services::event_bus::{BusinessEvent, ShippedItem};

    /// 与 `BusinessEvent` 字段一一对应的可序列化结构
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    pub enum EventPayload {
        PurchaseReceiptCompleted {
            receipt_id: i32,
            order_id: i32,
            supplier_id: i32,
        },
        SalesOrderShipped {
            order_id: i32,
            customer_id: i32,
            items: Vec<ShippedItem>,
        },
        // B-P1-4 修复（批次 361 v13 复审）：销售订单状态变更事件
        SalesOrderSubmitted {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderApproved {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderCompleted {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderCancelled {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderRejected {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        PaymentCompleted {
            payment_id: i32,
            invoice_id: i32,
            amount: Decimal,
            user_id: i32,
        },
        CollectionCompleted {
            collection_id: i32,
            invoice_id: Option<i32>,
            amount: Decimal,
            /// P1 1-1 修复（批次 78 v1 复审）：收款操作人 ID
            user_id: i32,
        },
        PurchaseOrderApproved {
            order_id: i32,
            supplier_id: i32,
        },
        InventoryCountCompleted {
            count_id: i32,
            variance_count: i32,
        },
        BpmProcessFinished {
            business_type: String,
            business_id: i32,
            approved: bool,
            /// P2 5-18 修复：审批人 ID（从 BPM 事件 payload 携带）
            approver_id: i32,
        },
        LowStockAlert {
            product_id: i32,
            warehouse_id: i32,
            current_quantity: Decimal,
            reorder_point: Decimal,
            reorder_quantity: Decimal,
        },
        FinancialIndicatorUpdate {
            period: String,
            trigger_source: String,
        },
        MaterialShortageAlert {
            material_id: i32,
            material_name: String,
            material_code: String,
            required_quantity: Decimal,
            available_quantity: Decimal,
            shortage_quantity: Decimal,
            shortage_level: String,
            affected_orders_count: i32,
        },
        InventoryTransactionCreated {
            transaction_id: i32,
            transaction_type: String,
            product_id: i32,
            warehouse_id: i32,
            quantity_meters: Decimal,
            quantity_kg: Decimal,
            source_bill_type: Option<String>,
            source_bill_no: Option<String>,
            source_bill_id: Option<i32>,
            batch_no: String,
            color_no: String,
            created_by: Option<i32>,
        },
        // B-P1-3 修复（批次 384 v13 复审）：客户/供应商主数据变更事件
        CustomerUpdated {
            customer_id: i32,
            customer_name: String,
            user_id: i32,
        },
        SupplierUpdated {
            supplier_id: i32,
            supplier_name: String,
            user_id: i32,
        },
        // v14 批次 420 修复 T-P1-3：染色完成/质检完成事件
        DyeBatchCompleted {
            batch_id: i32,
            batch_no: String,
            color_no: Option<String>,
            greige_fabric_id: Option<i32>,
            planned_quantity: Option<Decimal>,
            completed_by: Option<i32>,
        },
        QualityInspectionCompleted {
            inspection_id: i32,
            batch_id: Option<i32>,
            product_id: i32,
            result: String,
            inspector_id: Option<i32>,
        },
        // V15 Batch05-P1-3：面料行业 6 个核心业务事件
        ProcessStepReported {
            step_record_id: i32,
            flow_card_id: i32,
            route_code: String,
            operator_id: Option<i32>,
            started_at: Option<chrono::DateTime<chrono::Utc>>,
            completed_at: Option<chrono::DateTime<chrono::Utc>>,
            quantity: Option<Decimal>,
        },
        DyeBatchStatusChanged {
            batch_id: i32,
            batch_no: String,
            from_status: String,
            to_status: String,
            transition_code: String,
            operator_id: Option<i32>,
            transition_at: chrono::DateTime<chrono::Utc>,
        },
        FabricInspectionGraded {
            inspection_id: i32,
            batch_id: Option<i32>,
            grade: String,
            handling_method: Option<String>,
            inspector_id: Option<i32>,
        },
        ProductionQuantityReported {
            step_record_id: i32,
            flow_card_id: i32,
            operator_id: Option<i32>,
            actual_quantity: Decimal,
            qualified_quantity: Decimal,
        },
        EnergyConsumptionRecorded {
            record_id: i32,
            workshop: Option<String>,
            meter_type: String,
            consumption: Decimal,
            cost: Decimal,
            recorded_at: chrono::DateTime<chrono::Utc>,
        },
        ColorCardIssued {
            issue_id: i32,
            color_card_id: i32,
            customer_id: Option<i32>,
            issued_by: Option<i32>,
            issued_at: chrono::DateTime<chrono::Utc>,
        },
        // V15 Batch04-P1-3：工资确认/发放事件
        WageConfirmed {
            wage_record_id: i32,
            record_no: String,
            total_amount: Decimal,
            confirmed_by: i32,
        },
        WagePaid {
            wage_record_id: i32,
            record_no: String,
            total_amount: Decimal,
            paid_by: i32,
        },
        // V15 Batch04-P1-5：委外加工业务事件
        OutsourcingMaterialIssued {
            order_id: i32,
            order_no: String,
            order_type: String,
            supplier_id: i32,
            issue_quantity: Decimal,
            voucher_no_issue: Option<String>,
        },
        OutsourcingProcessingRecorded {
            order_id: i32,
            order_no: String,
            order_type: String,
            supplier_id: i32,
        },
        OutsourcingOrderSettled {
            order_id: i32,
            order_no: String,
            order_type: String,
            supplier_id: i32,
            processing_fee: Decimal,
            freight_fee: Decimal,
            normal_loss: Decimal,
            abnormal_loss: Decimal,
            total_cost: Decimal,
            unit_cost: Decimal,
            voucher_no_fee: Option<String>,
        },
        OutsourcingOrderCompleted {
            order_id: i32,
            order_no: String,
            order_type: String,
            supplier_id: i32,
            return_quantity: Decimal,
            voucher_no_receipt: Option<String>,
        },
        // V15 Batch04-P1-6：业务模式切换事件
        BusinessModeChanged {
            mode_id: i32,
            mode_code: String,
            mode_name: String,
            changed_by: i32,
        },
        OrderBusinessModeLinked {
            document_type: String,
            document_id: i32,
            document_no: String,
            mode_id: i32,
            mode_code: String,
            mode_name: String,
        },
    }

    impl From<&BusinessEvent> for EventPayload {
        fn from(event: &BusinessEvent) -> Self {
            use BusinessEvent::*;
            match event {
                SalesOrderShipped { .. }
                | SalesOrderSubmitted { .. }
                | SalesOrderApproved { .. }
                | SalesOrderCompleted { .. }
                | SalesOrderCancelled { .. }
                | SalesOrderRejected { .. } => from_sales_events(event),
                PurchaseReceiptCompleted { .. } | PurchaseOrderApproved { .. } => {
                    from_purchase_events(event)
                }
                PaymentCompleted { .. } | CollectionCompleted { .. } => from_finance_events(event),
                InventoryCountCompleted { .. } | LowStockAlert { .. } => {
                    from_inventory_alert_events(event)
                }
                InventoryTransactionCreated { .. } => from_inventory_transaction_event(event),
                BpmProcessFinished { .. } | FinancialIndicatorUpdate { .. } => {
                    from_process_events(event)
                }
                MaterialShortageAlert { .. }
                | CustomerUpdated { .. }
                | SupplierUpdated { .. }
                | DyeBatchCompleted { .. }
                | QualityInspectionCompleted { .. }
                | ProcessStepReported { .. }
                | DyeBatchStatusChanged { .. }
                | FabricInspectionGraded { .. }
                | ProductionQuantityReported { .. }
                | EnergyConsumptionRecorded { .. }
                | ColorCardIssued { .. }
                | WageConfirmed { .. }
                | WagePaid { .. }
                | OutsourcingMaterialIssued { .. }
                | OutsourcingProcessingRecorded { .. }
                | OutsourcingOrderSettled { .. }
                | OutsourcingOrderCompleted { .. }
                | BusinessModeChanged { .. }
                | OrderBusinessModeLinked { .. } => from_other_events(event),
            }
        }
    }

    /// 销售类事件（6 个 variant）转换为 EventPayload
    fn from_sales_events(event: &BusinessEvent) -> EventPayload {
        match event {
            BusinessEvent::SalesOrderShipped {
                order_id,
                customer_id,
                items,
            } => EventPayload::SalesOrderShipped {
                order_id: *order_id,
                customer_id: *customer_id,
                items: items.clone(),
            },
            BusinessEvent::SalesOrderSubmitted {
                order_id,
                customer_id,
                user_id,
            } => EventPayload::SalesOrderSubmitted {
                order_id: *order_id,
                customer_id: *customer_id,
                user_id: *user_id,
            },
            BusinessEvent::SalesOrderApproved {
                order_id,
                customer_id,
                user_id,
            } => EventPayload::SalesOrderApproved {
                order_id: *order_id,
                customer_id: *customer_id,
                user_id: *user_id,
            },
            BusinessEvent::SalesOrderCompleted {
                order_id,
                customer_id,
                user_id,
            } => EventPayload::SalesOrderCompleted {
                order_id: *order_id,
                customer_id: *customer_id,
                user_id: *user_id,
            },
            BusinessEvent::SalesOrderCancelled {
                order_id,
                customer_id,
                user_id,
            } => EventPayload::SalesOrderCancelled {
                order_id: *order_id,
                customer_id: *customer_id,
                user_id: *user_id,
            },
            BusinessEvent::SalesOrderRejected {
                order_id,
                customer_id,
                user_id,
            } => EventPayload::SalesOrderRejected {
                order_id: *order_id,
                customer_id: *customer_id,
                user_id: *user_id,
            },
            _ => unreachable!("from_sales_events 仅处理销售类事件"),
        }
    }

    /// 采购类事件（2 个 variant）转换为 EventPayload
    fn from_purchase_events(event: &BusinessEvent) -> EventPayload {
        match event {
            BusinessEvent::PurchaseReceiptCompleted {
                receipt_id,
                order_id,
                supplier_id,
            } => EventPayload::PurchaseReceiptCompleted {
                receipt_id: *receipt_id,
                order_id: *order_id,
                supplier_id: *supplier_id,
            },
            BusinessEvent::PurchaseOrderApproved {
                order_id,
                supplier_id,
            } => EventPayload::PurchaseOrderApproved {
                order_id: *order_id,
                supplier_id: *supplier_id,
            },
            _ => unreachable!("from_purchase_events 仅处理采购类事件"),
        }
    }

    /// 财务类事件（2 个 variant）转换为 EventPayload
    fn from_finance_events(event: &BusinessEvent) -> EventPayload {
        match event {
            BusinessEvent::PaymentCompleted {
                payment_id,
                invoice_id,
                amount,
                user_id,
            } => EventPayload::PaymentCompleted {
                payment_id: *payment_id,
                invoice_id: *invoice_id,
                amount: *amount,
                user_id: *user_id,
            },
            BusinessEvent::CollectionCompleted {
                collection_id,
                invoice_id,
                amount,
                user_id,
            } => EventPayload::CollectionCompleted {
                collection_id: *collection_id,
                invoice_id: *invoice_id,
                amount: *amount,
                user_id: *user_id,
            },
            _ => unreachable!("from_finance_events 仅处理财务类事件"),
        }
    }

    /// 库存计数/低库存告警类事件（2 个 variant）转换为 EventPayload
    fn from_inventory_alert_events(event: &BusinessEvent) -> EventPayload {
        match event {
            BusinessEvent::InventoryCountCompleted {
                count_id,
                variance_count,
            } => EventPayload::InventoryCountCompleted {
                count_id: *count_id,
                variance_count: *variance_count,
            },
            BusinessEvent::LowStockAlert {
                product_id,
                warehouse_id,
                current_quantity,
                reorder_point,
                reorder_quantity,
            } => EventPayload::LowStockAlert {
                product_id: *product_id,
                warehouse_id: *warehouse_id,
                current_quantity: *current_quantity,
                reorder_point: *reorder_point,
                reorder_quantity: *reorder_quantity,
            },
            _ => unreachable!("from_inventory_alert_events 仅处理库存计数/低库存告警类事件"),
        }
    }

    /// 库存交易事件（InventoryTransactionCreated）转换为 EventPayload
    fn from_inventory_transaction_event(event: &BusinessEvent) -> EventPayload {
        if let BusinessEvent::InventoryTransactionCreated {
            transaction_id,
            transaction_type,
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = event
        {
            EventPayload::InventoryTransactionCreated {
                transaction_id: *transaction_id,
                transaction_type: transaction_type.clone(),
                product_id: *product_id,
                warehouse_id: *warehouse_id,
                quantity_meters: *quantity_meters,
                quantity_kg: *quantity_kg,
                source_bill_type: source_bill_type.clone(),
                source_bill_no: source_bill_no.clone(),
                source_bill_id: *source_bill_id,
                batch_no: batch_no.clone(),
                color_no: color_no.clone(),
                created_by: *created_by,
            }
        } else {
            unreachable!("from_inventory_transaction_event 仅处理 InventoryTransactionCreated")
        }
    }

    /// 流程类事件（BpmProcessFinished/FinancialIndicatorUpdate）转换为 EventPayload
    fn from_process_events(event: &BusinessEvent) -> EventPayload {
        match event {
            BusinessEvent::BpmProcessFinished {
                business_type,
                business_id,
                approved,
                approver_id,
            } => EventPayload::BpmProcessFinished {
                business_type: business_type.clone(),
                business_id: *business_id,
                approved: *approved,
                approver_id: *approver_id,
            },
            BusinessEvent::FinancialIndicatorUpdate {
                period,
                trigger_source,
            } => EventPayload::FinancialIndicatorUpdate {
                period: period.clone(),
                trigger_source: trigger_source.clone(),
            },
            _ => unreachable!("from_process_events 仅处理流程类事件"),
        }
    }

    /// 主数据/缺料/染色质量类事件（5 个 variant）转换为 EventPayload
    fn from_other_events(event: &BusinessEvent) -> EventPayload {
        match event {
            BusinessEvent::MaterialShortageAlert {
                material_id,
                material_name,
                material_code,
                required_quantity,
                available_quantity,
                shortage_quantity,
                shortage_level,
                affected_orders_count,
            } => EventPayload::MaterialShortageAlert {
                material_id: *material_id,
                material_name: material_name.clone(),
                material_code: material_code.clone(),
                required_quantity: *required_quantity,
                available_quantity: *available_quantity,
                shortage_quantity: *shortage_quantity,
                shortage_level: shortage_level.clone(),
                affected_orders_count: *affected_orders_count,
            },
            BusinessEvent::CustomerUpdated {
                customer_id,
                customer_name,
                user_id,
            } => EventPayload::CustomerUpdated {
                customer_id: *customer_id,
                customer_name: customer_name.clone(),
                user_id: *user_id,
            },
            BusinessEvent::SupplierUpdated {
                supplier_id,
                supplier_name,
                user_id,
            } => EventPayload::SupplierUpdated {
                supplier_id: *supplier_id,
                supplier_name: supplier_name.clone(),
                user_id: *user_id,
            },
            BusinessEvent::DyeBatchCompleted {
                batch_id,
                batch_no,
                color_no,
                greige_fabric_id,
                planned_quantity,
                completed_by,
            } => EventPayload::DyeBatchCompleted {
                batch_id: *batch_id,
                batch_no: batch_no.clone(),
                color_no: color_no.clone(),
                greige_fabric_id: *greige_fabric_id,
                planned_quantity: *planned_quantity,
                completed_by: *completed_by,
            },
            BusinessEvent::QualityInspectionCompleted {
                inspection_id,
                batch_id,
                product_id,
                result,
                inspector_id,
            } => EventPayload::QualityInspectionCompleted {
                inspection_id: *inspection_id,
                batch_id: *batch_id,
                product_id: *product_id,
                result: result.clone(),
                inspector_id: *inspector_id,
            },
            BusinessEvent::ProcessStepReported {
                step_record_id,
                flow_card_id,
                route_code,
                operator_id,
                started_at,
                completed_at,
                quantity,
            } => EventPayload::ProcessStepReported {
                step_record_id: *step_record_id,
                flow_card_id: *flow_card_id,
                route_code: route_code.clone(),
                operator_id: *operator_id,
                started_at: *started_at,
                completed_at: *completed_at,
                quantity: *quantity,
            },
            BusinessEvent::DyeBatchStatusChanged {
                batch_id,
                batch_no,
                from_status,
                to_status,
                transition_code,
                operator_id,
                transition_at,
            } => EventPayload::DyeBatchStatusChanged {
                batch_id: *batch_id,
                batch_no: batch_no.clone(),
                from_status: from_status.clone(),
                to_status: to_status.clone(),
                transition_code: transition_code.clone(),
                operator_id: *operator_id,
                transition_at: *transition_at,
            },
            BusinessEvent::FabricInspectionGraded {
                inspection_id,
                batch_id,
                grade,
                handling_method,
                inspector_id,
            } => EventPayload::FabricInspectionGraded {
                inspection_id: *inspection_id,
                batch_id: *batch_id,
                grade: grade.clone(),
                handling_method: handling_method.clone(),
                inspector_id: *inspector_id,
            },
            BusinessEvent::ProductionQuantityReported {
                step_record_id,
                flow_card_id,
                operator_id,
                actual_quantity,
                qualified_quantity,
            } => EventPayload::ProductionQuantityReported {
                step_record_id: *step_record_id,
                flow_card_id: *flow_card_id,
                operator_id: *operator_id,
                actual_quantity: *actual_quantity,
                qualified_quantity: *qualified_quantity,
            },
            BusinessEvent::EnergyConsumptionRecorded {
                record_id,
                workshop,
                meter_type,
                consumption,
                cost,
                recorded_at,
            } => EventPayload::EnergyConsumptionRecorded {
                record_id: *record_id,
                workshop: workshop.clone(),
                meter_type: meter_type.clone(),
                consumption: *consumption,
                cost: *cost,
                recorded_at: *recorded_at,
            },
            BusinessEvent::ColorCardIssued {
                issue_id,
                color_card_id,
                customer_id,
                issued_by,
                issued_at,
            } => EventPayload::ColorCardIssued {
                issue_id: *issue_id,
                color_card_id: *color_card_id,
                customer_id: *customer_id,
                issued_by: *issued_by,
                issued_at: *issued_at,
            },
            BusinessEvent::WageConfirmed {
                wage_record_id,
                record_no,
                total_amount,
                confirmed_by,
            } => EventPayload::WageConfirmed {
                wage_record_id: *wage_record_id,
                record_no: record_no.clone(),
                total_amount: *total_amount,
                confirmed_by: *confirmed_by,
            },
            BusinessEvent::WagePaid {
                wage_record_id,
                record_no,
                total_amount,
                paid_by,
            } => EventPayload::WagePaid {
                wage_record_id: *wage_record_id,
                record_no: record_no.clone(),
                total_amount: *total_amount,
                paid_by: *paid_by,
            },
            BusinessEvent::OutsourcingMaterialIssued {
                order_id,
                order_no,
                order_type,
                supplier_id,
                issue_quantity,
                voucher_no_issue,
            } => EventPayload::OutsourcingMaterialIssued {
                order_id: *order_id,
                order_no: order_no.clone(),
                order_type: order_type.clone(),
                supplier_id: *supplier_id,
                issue_quantity: *issue_quantity,
                voucher_no_issue: voucher_no_issue.clone(),
            },
            BusinessEvent::OutsourcingProcessingRecorded {
                order_id,
                order_no,
                order_type,
                supplier_id,
            } => EventPayload::OutsourcingProcessingRecorded {
                order_id: *order_id,
                order_no: order_no.clone(),
                order_type: order_type.clone(),
                supplier_id: *supplier_id,
            },
            BusinessEvent::OutsourcingOrderSettled {
                order_id,
                order_no,
                order_type,
                supplier_id,
                processing_fee,
                freight_fee,
                normal_loss,
                abnormal_loss,
                total_cost,
                unit_cost,
                voucher_no_fee,
            } => EventPayload::OutsourcingOrderSettled {
                order_id: *order_id,
                order_no: order_no.clone(),
                order_type: order_type.clone(),
                supplier_id: *supplier_id,
                processing_fee: *processing_fee,
                freight_fee: *freight_fee,
                normal_loss: *normal_loss,
                abnormal_loss: *abnormal_loss,
                total_cost: *total_cost,
                unit_cost: *unit_cost,
                voucher_no_fee: voucher_no_fee.clone(),
            },
            BusinessEvent::OutsourcingOrderCompleted {
                order_id,
                order_no,
                order_type,
                supplier_id,
                return_quantity,
                voucher_no_receipt,
            } => EventPayload::OutsourcingOrderCompleted {
                order_id: *order_id,
                order_no: order_no.clone(),
                order_type: order_type.clone(),
                supplier_id: *supplier_id,
                return_quantity: *return_quantity,
                voucher_no_receipt: voucher_no_receipt.clone(),
            },
            BusinessEvent::BusinessModeChanged {
                mode_id,
                mode_code,
                mode_name,
                changed_by,
            } => EventPayload::BusinessModeChanged {
                mode_id: *mode_id,
                mode_code: mode_code.clone(),
                mode_name: mode_name.clone(),
                changed_by: *changed_by,
            },
            BusinessEvent::OrderBusinessModeLinked {
                document_type,
                document_id,
                document_no,
                mode_id,
                mode_code,
                mode_name,
            } => EventPayload::OrderBusinessModeLinked {
                document_type: document_type.clone(),
                document_id: *document_id,
                document_no: document_no.clone(),
                mode_id: *mode_id,
                mode_code: mode_code.clone(),
                mode_name: mode_name.clone(),
            },
            _ => unreachable!("from_other_events 仅处理主数据/缺料/染色质量类事件"),
        }
    }

    impl TryFrom<EventPayload> for BusinessEvent {
        type Error = String;
        fn try_from(p: EventPayload) -> Result<Self, Self::Error> {
            use EventPayload::*;
            Ok(match p {
                SalesOrderShipped { .. }
                | SalesOrderSubmitted { .. }
                | SalesOrderApproved { .. }
                | SalesOrderCompleted { .. }
                | SalesOrderCancelled { .. }
                | SalesOrderRejected { .. } => to_sales_events(p)?,
                PurchaseReceiptCompleted { .. } | PurchaseOrderApproved { .. } => {
                    to_purchase_events(p)?
                }
                PaymentCompleted { .. } | CollectionCompleted { .. } => to_finance_events(p)?,
                InventoryCountCompleted { .. } | LowStockAlert { .. } => {
                    to_inventory_alert_events(p)?
                }
                InventoryTransactionCreated { .. } => to_inventory_transaction_event(p)?,
                BpmProcessFinished { .. } | FinancialIndicatorUpdate { .. } => {
                    to_process_events(p)?
                }
                MaterialShortageAlert { .. }
                | CustomerUpdated { .. }
                | SupplierUpdated { .. }
                | DyeBatchCompleted { .. }
                | QualityInspectionCompleted { .. }
                | ProcessStepReported { .. }
                | DyeBatchStatusChanged { .. }
                | FabricInspectionGraded { .. }
                | ProductionQuantityReported { .. }
                | EnergyConsumptionRecorded { .. }
                | ColorCardIssued { .. }
                | WageConfirmed { .. }
                | WagePaid { .. }
                | OutsourcingMaterialIssued { .. }
                | OutsourcingProcessingRecorded { .. }
                | OutsourcingOrderSettled { .. }
                | OutsourcingOrderCompleted { .. }
                | BusinessModeChanged { .. }
                | OrderBusinessModeLinked { .. } => to_other_events(p)?,
            })
        }
    }

    /// 销售类 EventPayload 反向转换为 BusinessEvent
    fn to_sales_events(p: EventPayload) -> Result<BusinessEvent, String> {
        Ok(match p {
            EventPayload::SalesOrderShipped {
                order_id,
                customer_id,
                items,
            } => BusinessEvent::SalesOrderShipped {
                order_id,
                customer_id,
                items,
            },
            EventPayload::SalesOrderSubmitted {
                order_id,
                customer_id,
                user_id,
            } => BusinessEvent::SalesOrderSubmitted {
                order_id,
                customer_id,
                user_id,
            },
            EventPayload::SalesOrderApproved {
                order_id,
                customer_id,
                user_id,
            } => BusinessEvent::SalesOrderApproved {
                order_id,
                customer_id,
                user_id,
            },
            EventPayload::SalesOrderCompleted {
                order_id,
                customer_id,
                user_id,
            } => BusinessEvent::SalesOrderCompleted {
                order_id,
                customer_id,
                user_id,
            },
            EventPayload::SalesOrderCancelled {
                order_id,
                customer_id,
                user_id,
            } => BusinessEvent::SalesOrderCancelled {
                order_id,
                customer_id,
                user_id,
            },
            EventPayload::SalesOrderRejected {
                order_id,
                customer_id,
                user_id,
            } => BusinessEvent::SalesOrderRejected {
                order_id,
                customer_id,
                user_id,
            },
            _ => return Err("to_sales_events 仅处理销售类 EventPayload".to_string()),
        })
    }

    /// 采购类 EventPayload 反向转换为 BusinessEvent
    fn to_purchase_events(p: EventPayload) -> Result<BusinessEvent, String> {
        Ok(match p {
            EventPayload::PurchaseReceiptCompleted {
                receipt_id,
                order_id,
                supplier_id,
            } => BusinessEvent::PurchaseReceiptCompleted {
                receipt_id,
                order_id,
                supplier_id,
            },
            EventPayload::PurchaseOrderApproved {
                order_id,
                supplier_id,
            } => BusinessEvent::PurchaseOrderApproved {
                order_id,
                supplier_id,
            },
            _ => return Err("to_purchase_events 仅处理采购类 EventPayload".to_string()),
        })
    }

    /// 财务类 EventPayload 反向转换为 BusinessEvent
    fn to_finance_events(p: EventPayload) -> Result<BusinessEvent, String> {
        Ok(match p {
            EventPayload::PaymentCompleted {
                payment_id,
                invoice_id,
                amount,
                user_id,
            } => BusinessEvent::PaymentCompleted {
                payment_id,
                invoice_id,
                amount,
                user_id,
            },
            EventPayload::CollectionCompleted {
                collection_id,
                invoice_id,
                amount,
                user_id,
            } => BusinessEvent::CollectionCompleted {
                collection_id,
                invoice_id,
                amount,
                user_id,
            },
            _ => return Err("to_finance_events 仅处理财务类 EventPayload".to_string()),
        })
    }

    /// 库存计数/低库存告警类 EventPayload 反向转换为 BusinessEvent
    fn to_inventory_alert_events(p: EventPayload) -> Result<BusinessEvent, String> {
        Ok(match p {
            EventPayload::InventoryCountCompleted {
                count_id,
                variance_count,
            } => BusinessEvent::InventoryCountCompleted {
                count_id,
                variance_count,
            },
            EventPayload::LowStockAlert {
                product_id,
                warehouse_id,
                current_quantity,
                reorder_point,
                reorder_quantity,
            } => BusinessEvent::LowStockAlert {
                product_id,
                warehouse_id,
                current_quantity,
                reorder_point,
                reorder_quantity,
            },
            _ => {
                return Err(
                    "to_inventory_alert_events 仅处理库存计数/低库存告警类 EventPayload"
                        .to_string(),
                )
            }
        })
    }

    /// 库存交易 EventPayload 反向转换为 BusinessEvent
    fn to_inventory_transaction_event(p: EventPayload) -> Result<BusinessEvent, String> {
        if let EventPayload::InventoryTransactionCreated {
            transaction_id,
            transaction_type,
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = p
        {
            Ok(BusinessEvent::InventoryTransactionCreated {
                transaction_id,
                transaction_type,
                product_id,
                warehouse_id,
                quantity_meters,
                quantity_kg,
                source_bill_type,
                source_bill_no,
                source_bill_id,
                batch_no,
                color_no,
                created_by,
            })
        } else {
            Err("to_inventory_transaction_event 仅处理 InventoryTransactionCreated".to_string())
        }
    }

    /// 流程类 EventPayload 反向转换为 BusinessEvent
    fn to_process_events(p: EventPayload) -> Result<BusinessEvent, String> {
        Ok(match p {
            EventPayload::BpmProcessFinished {
                business_type,
                business_id,
                approved,
                approver_id,
            } => BusinessEvent::BpmProcessFinished {
                business_type,
                business_id,
                approved,
                approver_id,
            },
            EventPayload::FinancialIndicatorUpdate {
                period,
                trigger_source,
            } => BusinessEvent::FinancialIndicatorUpdate {
                period,
                trigger_source,
            },
            _ => return Err("to_process_events 仅处理流程类 EventPayload".to_string()),
        })
    }

    /// 主数据/缺料/染色质量类 EventPayload 反向转换为 BusinessEvent
    fn to_other_events(p: EventPayload) -> Result<BusinessEvent, String> {
        Ok(match p {
            EventPayload::MaterialShortageAlert {
                material_id,
                material_name,
                material_code,
                required_quantity,
                available_quantity,
                shortage_quantity,
                shortage_level,
                affected_orders_count,
            } => BusinessEvent::MaterialShortageAlert {
                material_id,
                material_name,
                material_code,
                required_quantity,
                available_quantity,
                shortage_quantity,
                shortage_level,
                affected_orders_count,
            },
            EventPayload::CustomerUpdated {
                customer_id,
                customer_name,
                user_id,
            } => BusinessEvent::CustomerUpdated {
                customer_id,
                customer_name,
                user_id,
            },
            EventPayload::SupplierUpdated {
                supplier_id,
                supplier_name,
                user_id,
            } => BusinessEvent::SupplierUpdated {
                supplier_id,
                supplier_name,
                user_id,
            },
            EventPayload::DyeBatchCompleted {
                batch_id,
                batch_no,
                color_no,
                greige_fabric_id,
                planned_quantity,
                completed_by,
            } => BusinessEvent::DyeBatchCompleted {
                batch_id,
                batch_no,
                color_no,
                greige_fabric_id,
                planned_quantity,
                completed_by,
            },
            EventPayload::QualityInspectionCompleted {
                inspection_id,
                batch_id,
                product_id,
                result,
                inspector_id,
            } => BusinessEvent::QualityInspectionCompleted {
                inspection_id,
                batch_id,
                product_id,
                result,
                inspector_id,
            },
            EventPayload::ProcessStepReported {
                step_record_id,
                flow_card_id,
                route_code,
                operator_id,
                started_at,
                completed_at,
                quantity,
            } => BusinessEvent::ProcessStepReported {
                step_record_id,
                flow_card_id,
                route_code,
                operator_id,
                started_at,
                completed_at,
                quantity,
            },
            EventPayload::DyeBatchStatusChanged {
                batch_id,
                batch_no,
                from_status,
                to_status,
                transition_code,
                operator_id,
                transition_at,
            } => BusinessEvent::DyeBatchStatusChanged {
                batch_id,
                batch_no,
                from_status,
                to_status,
                transition_code,
                operator_id,
                transition_at,
            },
            EventPayload::FabricInspectionGraded {
                inspection_id,
                batch_id,
                grade,
                handling_method,
                inspector_id,
            } => BusinessEvent::FabricInspectionGraded {
                inspection_id,
                batch_id,
                grade,
                handling_method,
                inspector_id,
            },
            EventPayload::ProductionQuantityReported {
                step_record_id,
                flow_card_id,
                operator_id,
                actual_quantity,
                qualified_quantity,
            } => BusinessEvent::ProductionQuantityReported {
                step_record_id,
                flow_card_id,
                operator_id,
                actual_quantity,
                qualified_quantity,
            },
            EventPayload::EnergyConsumptionRecorded {
                record_id,
                workshop,
                meter_type,
                consumption,
                cost,
                recorded_at,
            } => BusinessEvent::EnergyConsumptionRecorded {
                record_id,
                workshop,
                meter_type,
                consumption,
                cost,
                recorded_at,
            },
            EventPayload::ColorCardIssued {
                issue_id,
                color_card_id,
                customer_id,
                issued_by,
                issued_at,
            } => BusinessEvent::ColorCardIssued {
                issue_id,
                color_card_id,
                customer_id,
                issued_by,
                issued_at,
            },
            EventPayload::WageConfirmed {
                wage_record_id,
                record_no,
                total_amount,
                confirmed_by,
            } => BusinessEvent::WageConfirmed {
                wage_record_id,
                record_no,
                total_amount,
                confirmed_by,
            },
            EventPayload::WagePaid {
                wage_record_id,
                record_no,
                total_amount,
                paid_by,
            } => BusinessEvent::WagePaid {
                wage_record_id,
                record_no,
                total_amount,
                paid_by,
            },
            EventPayload::OutsourcingMaterialIssued {
                order_id,
                order_no,
                order_type,
                supplier_id,
                issue_quantity,
                voucher_no_issue,
            } => BusinessEvent::OutsourcingMaterialIssued {
                order_id,
                order_no,
                order_type,
                supplier_id,
                issue_quantity,
                voucher_no_issue,
            },
            EventPayload::OutsourcingProcessingRecorded {
                order_id,
                order_no,
                order_type,
                supplier_id,
            } => BusinessEvent::OutsourcingProcessingRecorded {
                order_id,
                order_no,
                order_type,
                supplier_id,
            },
            EventPayload::OutsourcingOrderSettled {
                order_id,
                order_no,
                order_type,
                supplier_id,
                processing_fee,
                freight_fee,
                normal_loss,
                abnormal_loss,
                total_cost,
                unit_cost,
                voucher_no_fee,
            } => BusinessEvent::OutsourcingOrderSettled {
                order_id,
                order_no,
                order_type,
                supplier_id,
                processing_fee,
                freight_fee,
                normal_loss,
                abnormal_loss,
                total_cost,
                unit_cost,
                voucher_no_fee,
            },
            EventPayload::OutsourcingOrderCompleted {
                order_id,
                order_no,
                order_type,
                supplier_id,
                return_quantity,
                voucher_no_receipt,
            } => BusinessEvent::OutsourcingOrderCompleted {
                order_id,
                order_no,
                order_type,
                supplier_id,
                return_quantity,
                voucher_no_receipt,
            },
            EventPayload::BusinessModeChanged {
                mode_id,
                mode_code,
                mode_name,
                changed_by,
            } => BusinessEvent::BusinessModeChanged {
                mode_id,
                mode_code,
                mode_name,
                changed_by,
            },
            EventPayload::OrderBusinessModeLinked {
                document_type,
                document_id,
                document_no,
                mode_id,
                mode_code,
                mode_name,
            } => BusinessEvent::OrderBusinessModeLinked {
                document_type,
                document_id,
                document_no,
                mode_id,
                mode_code,
                mode_name,
            },
            _ => {
                return Err("to_other_events 仅处理主数据/缺料/染色质量类 EventPayload".to_string())
            }
        })
    }
}

// 重导出 EventPayload 给外部直接访问
pub use payload_serde::EventPayload;
