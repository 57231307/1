// v14 批次 422 T-P1-7：染色完成→成本归集桥接服务
//
// 依据：fabric-industry-research.md §5.6 月末成本单价计算
// 业务规则：染色完成后自动创建成本归集草稿记录（status=draft），
// 关联 batch_no/color_no，后续由财务人员补充直接材料/直接人工/制造费用/外协加工费/染费明细并审核。
//
// 事件链路：DyeBatchCompleted 事件 → 本监听器 → 创建 cost_collection 草稿 → 财务人员补充审核

use crate::services::cost_collection_service::{
    CostCollectionService, CreateCostCollectionRequest,
};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::error::AppError;
use chrono::Utc;
use futures::FutureExt;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tracing::{error, info};

/// 染色成本桥接监听器 spawn 句柄
/// 保存句柄以便 shutdown 时 abort，避免 detached task 泄漏
static DYE_BATCH_COST_LISTENER_HANDLE: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(None);

/// 染色成本桥接服务
/// 监听 DyeBatchCompleted 事件，自动创建成本归集草稿记录
pub struct DyeBatchCostBridgeService;

impl DyeBatchCostBridgeService {
    /// 启动染色成本桥接监听器
    pub fn start_listener(db: Arc<DatabaseConnection>) {
        let mut receiver = EVENT_BUS.subscribe();

        let listener_handle = tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                // panic 隔离：单次事件处理 panic 不影响后续事件
                let result = AssertUnwindSafe(async {
                    if let BusinessEvent::DyeBatchCompleted {
                        batch_id,
                        ref batch_no,
                        ref color_no,
                        greige_fabric_id,
                        planned_quantity,
                        completed_by,
                    } = event
                    {
                        info!(
                            batch_id,
                            batch_no = %batch_no,
                            color_no = ?color_no,
                            "染色成本桥接监听器收到 DyeBatchCompleted 事件，开始创建成本归集草稿"
                        );

                        let bridge = DyeBatchCostBridgeServiceInternal::new(db.clone());
                        if let Err(e) = bridge
                            .handle_dye_batch_completed(
                                batch_id,
                                batch_no,
                                color_no.as_deref(),
                                greige_fabric_id,
                                planned_quantity,
                                completed_by,
                            )
                            .await
                        {
                            error!(
                                batch_id,
                                batch_no = %batch_no,
                                error = %e,
                                "染色成本桥接监听器处理 DyeBatchCompleted 事件失败"
                            );
                        }
                    }
                })
                .catch_unwind()
                .await;
                if let Err(panic_payload) = result {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    error!(
                        panic = %panic_msg,
                        "⚠ 染色成本桥接监听器 spawn panic 已被隔离，继续运行（不退出循环）"
                    );
                }
            }
        });

        // 保存句柄到全局 static
        if let Ok(mut guard) = DYE_BATCH_COST_LISTENER_HANDLE.lock() {
            *guard = Some(listener_handle);
        }
    }

    /// 优雅关闭染色成本桥接监听器
    /// abort 后台 spawn task，防止 detached task 泄漏。幂等：多次调用安全。
    pub fn shutdown_listener() {
        let handle = match DYE_BATCH_COST_LISTENER_HANDLE.lock() {
            Ok(mut guard) => guard.take(),
            Err(e) => {
                error!(error = %e, "DYE_BATCH_COST_LISTENER_HANDLE 锁中毒，无法关闭监听器");
                return;
            }
        };
        if let Some(h) = handle {
            h.abort();
            info!("染色成本桥接监听器 task 已关闭");
        }
    }
}

/// 内部实现结构，持有数据库连接
struct DyeBatchCostBridgeServiceInternal {
    db: Arc<DatabaseConnection>,
}

impl DyeBatchCostBridgeServiceInternal {
    fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 处理染色完成事件，创建成本归集草稿记录
    ///
    /// 创建一个 draft 状态的 cost_collection 记录，所有成本字段初始化为 0，
    /// 关联 batch_no/color_no/cost_object_no，后续由财务人员补充成本明细并审核。
    ///
    /// 依据：fabric-industry-research.md §5.6——染色完成后需归集染料/助剂/能耗成本到对应缸号
    async fn handle_dye_batch_completed(
        &self,
        batch_id: i32,
        batch_no: &str,
        color_no: Option<&str>,
        _greige_fabric_id: Option<i32>,
        _planned_quantity: Option<Decimal>,
        completed_by: Option<i32>,
    ) -> Result<(), AppError> {
        let cost_service = CostCollectionService::new(self.db.clone());

        // 构造成本归集草稿请求
        // 所有成本字段初始化为 0，后续由财务人员补充
        let req = CreateCostCollectionRequest {
            collection_date: Utc::now().date_naive(),
            cost_object_type: Some("dye_batch".to_string()),
            cost_object_id: Some(batch_id),
            cost_object_no: Some(batch_no.to_string()),
            batch_no: Some(batch_no.to_string()),
            color_no: color_no.map(|s| s.to_string()),
            // dye_lot_no 暂为 None，dye_batch 表当前无此字段，后续批次补全
            dye_lot_no: None,
            workshop: Some("染色车间".to_string()),
            direct_material: Decimal::ZERO,
            direct_labor: Decimal::ZERO,
            manufacturing_overhead: Decimal::ZERO,
            processing_fee: Decimal::ZERO,
            dyeing_fee: Decimal::ZERO,
            output_quantity_meters: None,
            output_quantity_kg: None,
        };

        let result = cost_service
            .create(req, completed_by.unwrap_or(0))
            .await?;

        info!(
            collection_no = %result.collection_no,
            batch_no = %batch_no,
            "染色完成成本归集草稿创建成功，待财务人员补充成本明细并审核"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试_染色成本桥接服务_静态方法存在性
    ///
    /// 验证 start_listener / shutdown_listener 方法可调用（编译时检查）。
    /// 实际事件监听需数据库连接，标注为编译时检查。
    #[test]
    fn 测试_染色成本桥接服务_静态方法存在性() {
        // 验证函数指针可获取（编译时检查）
        let _start: fn(Arc<DatabaseConnection>) = DyeBatchCostBridgeService::start_listener;
        let _shutdown: fn() = DyeBatchCostBridgeService::shutdown_listener;
    }
}
