// v14 批次 422 T-P1-7：染色完成→成本归集桥接服务
//
// 依据：.monkeycode/docs/research/fabric-industry-research.md §5.6 月末成本单价计算
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
use sea_orm::{DatabaseConnection, EntityTrait};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::models::dye_batch;

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
/// （pub：完工产出回填分母逻辑需被同域契约测直接驱动，见
/// tests/contract_wave5_dye_batch_complete_actual_output_test.rs）
pub struct DyeBatchCostBridgeServiceInternal {
    db: Arc<DatabaseConnection>,
}

impl DyeBatchCostBridgeServiceInternal {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 处理染色完成事件，创建成本归集草稿记录
    /// 创建一个 draft 状态的 cost_collection 记录，所有成本字段初始化为 0，；关联 batch_no/color_no/cost_object_no，后续由财务人员补充成本明细并审核。；依据：.monkeycode/docs/research/fabric-industry-research.md §5.6——染色完成后需归集染料/助剂/能耗成本到对应缸号；V15 P0-F01：补全 dye_lot_no 关联；原实现 dye_lot_no 写死为 None（dye_batch 表无此字段），导致四维标识断裂、；成本归集无法关联到具体染缸号。修复后通过 batch_id 查询 dye_batch 表获取 dye_lot_no。
    ///
    /// 任务 #168：产量分母回填——dye_lot_no 与 actual_output_kg/actual_output_m 取自同一
    /// dye_batch 行（完工端点强制必填后必有真值）。原实现把 output_quantity_kg/_meters
    /// 恒写 None，导致 draft 无分母、单位成本/能耗分摊算不出。单位成本本身仍由
    /// CostCollectionService 既有算法（total_cost / 产量，cost_collection_service.rs:88-103）
    /// 推导，不在本层另造算法；draft 阶段成本为 0 时其产出 0 也属该算法既有语义，
    /// 财务补充明细后 update 路径（cost_collection_service.rs:276-288）自动按分母重算。
    pub async fn handle_dye_batch_completed(
        &self,
        batch_id: i32,
        batch_no: &str,
        color_no: Option<&str>,
        _greige_fabric_id: Option<i32>,
        _planned_quantity: Option<Decimal>,
        completed_by: Option<i32>,
    ) -> Result<(), AppError> {
        let cost_service = CostCollectionService::new(self.db.clone());

        // V15 P0-F01 + 任务 #168：一次查询同取 dye_lot_no 与完工登记的实际产出（分母）
        // 术语：dye_lot_no（染色批号）≠ batch_no（缸号=染色批次号，同一概念不同叫法）
        // 历史数据回填为 'DEFAULT'，新数据由创建接口传入实际染色批号
        let (dye_lot_no, output_kg, output_m) = match dye_batch::Entity::find_by_id(batch_id)
            .one(&*self.db)
            .await
        {
            Ok(Some(batch)) => {
                if batch.actual_output_kg.is_none() || batch.actual_output_m.is_none() {
                    // 完工端点已强制必填，缺值只可能来自旁路写入/历史行：
                    // fail-visible 记 warn，分母保持真实 NULL，不造假值
                    warn!(
                        batch_id,
                        batch_no = %batch_no,
                        actual_output_kg = ?batch.actual_output_kg,
                        actual_output_m = ?batch.actual_output_m,
                        "dye_batch 完工实际产出缺失，成本归集产量分母落 NULL（不造假值）"
                    );
                }
                (
                    Some(batch.dye_lot_no),
                    batch.actual_output_kg,
                    batch.actual_output_m,
                )
            }
            Ok(None) => {
                warn!(
                    batch_id,
                    batch_no = %batch_no,
                    "查询 dye_batch 未找到记录，dye_lot_no 与产量分母降级为 None"
                );
                (None, None, None)
            }
            Err(e) => {
                warn!(
                    batch_id,
                    batch_no = %batch_no,
                    error = %e,
                    "查询 dye_batch 失败，dye_lot_no 与产量分母降级为 None（cost_collection 仍可创建）"
                );
                (None, None, None)
            }
        };

        // 构造成本归集草稿请求（纯函数，见 build_draft_cost_request 文档）
        let req = Self::build_draft_cost_request(
            dye_lot_no, output_kg, output_m, batch_id, batch_no, color_no,
        );

        let result = cost_service.create(req, completed_by.unwrap_or(0)).await?;

        info!(
            collection_no = %result.collection_no,
            batch_no = %batch_no,
            "染色完成成本归集草稿创建成功，待财务人员补充成本明细并审核"
        );

        Ok(())
    }

    /// 组装成本归集草稿请求（纯函数、无 IO）——产量分母回填的唯一落点（任务 #168）。
    /// 拆出独立函数的原因：cost_collection 单号生成走 pg_advisory_xact_lock
    /// （utils/crud_macro.rs::impl_generate_no → number_generator::lock_prefix），
    /// sqlite 契约测无法整链驱动 create()，故对"行值→请求分母"这一映射直接断言。
    pub fn build_draft_cost_request(
        dye_lot_no: Option<String>,
        output_kg: Option<Decimal>,
        output_m: Option<Decimal>,
        batch_id: i32,
        batch_no: &str,
        color_no: Option<&str>,
    ) -> CreateCostCollectionRequest {
        // 所有成本字段初始化为 0，后续由财务人员补充；产量分母取完工登记真值
        CreateCostCollectionRequest {
            collection_date: Utc::now().date_naive(),
            cost_object_type: Some("dye_batch".to_string()),
            cost_object_id: Some(batch_id),
            cost_object_no: Some(batch_no.to_string()),
            batch_no: Some(batch_no.to_string()),
            color_no: color_no.map(|s| s.to_string()),
            // V15 P0-F01：dye_lot_no 已通过 dye_batch 表查询获取（原写死 None）
            dye_lot_no,
            workshop: Some("染色车间".to_string()),
            direct_material: Decimal::ZERO,
            direct_labor: Decimal::ZERO,
            manufacturing_overhead: Decimal::ZERO,
            processing_fee: Decimal::ZERO,
            dyeing_fee: Decimal::ZERO,
            // 任务 #168：完工登记的实际产出回填分母（原恒写 None）
            output_quantity_meters: output_m,
            output_quantity_kg: output_kg,
        }
    }
}
