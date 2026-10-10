//! 库存调拨主流程服务（inv/move）
//!
//! 包含库存调拨单（Transfer）的核心 CRUD、状态机（pending → approved → shipped → completed）、
//! 单据号生成器等。原 `inventory_transfer_service.rs` 拆分而来。
//!
//! 子模块协作：
//! - 调拨明细与发出/接收的批次处理：batch.rs
//! - 调出仓库库存预检：stock.rs
//!
//! 注意：文件名 `inventory_move.rs`（非 `move.rs`），因为 `move` 是 Rust 关键字。

use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, JoinType, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationDef, RelationTrait,
    TransactionTrait,
};

use crate::models::dto::PageRequest;
use crate::models::inventory_transfer::{self, Entity as InventoryTransferEntity};
use crate::models::inventory_transfer_item::{self, Entity as InventoryTransferItemEntity};
use crate::models::status::inventory_transfer as transfer_status;
use crate::models::{product, user, warehouse};
// 行级数据权限工具
use crate::utils::PaginatedResponse;
use crate::utils::data_scope::{
    DataScopeContext, apply_data_scope, check_resource_owner_by_member_scope,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

use crate::services::inventory_deduction::require_outbound_dimensions;

use super::{
    CreateInventoryTransferRequest, InventoryTransferDetail, InventoryTransferItemDetail,
    InventoryTransferItemRequest, InventoryTransferService, InventoryTransferView,
    UpdateInventoryTransferRequest,
};

impl InventoryTransferService {
    /// 获取库存调拨列表
    /// 新增 data_scope 参数按行级数据权限过滤；本表无 department_id 列，Dept 按「归属人 ∈
    /// 可见部门成员集合」判定（owner 取 `created_by: Option<i32>`）。
    pub async fn list_transfers(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        from_warehouse_id: Option<i32>,
        to_warehouse_id: Option<i32>,
        transfer_no: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<PaginatedResponse<InventoryTransferDetail>, AppError> {
        // 单次 JOIN 富化查询：from/to 两个仓库须各自 LEFT JOIN warehouses，
        // 同一实体两次 JOIN 在 Postgres 下必须别名去歧义，故用 join_as + ("别名", 列) 输出不同列名；
        // created_by LEFT JOIN users 取真实展示名 real_name。JOIN 派生列在视图里均为 Option<String>。
        let created_by_rel: RelationDef = InventoryTransferEntity::belongs_to(user::Entity)
            .from(inventory_transfer::Column::CreatedBy)
            .to(user::Column::Id)
            .into();
        let mut query = InventoryTransferEntity::find()
            .expr_as(
                Expr::col((Alias::new("from_wh"), warehouse::Column::Name)),
                "from_warehouse_name",
            )
            .expr_as(
                Expr::col((Alias::new("to_wh"), warehouse::Column::Name)),
                "to_warehouse_name",
            )
            .column_as(user::Column::RealName, "created_by_name")
            .join_as(
                JoinType::LeftJoin,
                inventory_transfer::Relation::FromWarehouse.def(),
                "from_wh",
            )
            .join_as(
                JoinType::LeftJoin,
                inventory_transfer::Relation::ToWarehouse.def(),
                "to_wh",
            )
            .join(JoinType::LeftJoin, created_by_rel);

        // 应用过滤条件
        if let Some(s) = status {
            query = query.filter(inventory_transfer::Column::Status.eq(s));
        }
        if let Some(fid) = from_warehouse_id {
            query = query.filter(inventory_transfer::Column::FromWarehouseId.eq(fid));
        }
        if let Some(tid) = to_warehouse_id {
            query = query.filter(inventory_transfer::Column::ToWarehouseId.eq(tid));
        }
        if let Some(no) = transfer_no {
            query = query.filter(inventory_transfer::Column::TransferNo.contains(&no));
        }
        // 行级数据权限过滤
        // inventory_transfer 表无 department_id，Dept 退化为 Self，使用 created_by（Option<i32>）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                inventory_transfer::Column::CreatedBy,
                inventory_transfer::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
        }

        // 分页：单次查询取当页 + 总数（无逐行回查）
        let paginator = query
            .order_by(inventory_transfer::Column::CreatedAt, Order::Desc)
            .into_model::<InventoryTransferView>()
            .paginate(&*self.db, page_req.page_size);
        // 使用统一分页辅助函数，并行执行分页查询与总数统计
        let (views, total): (Vec<InventoryTransferView>, u64) =
            paginate_with_total(paginator, page_req.page).await?;

        // 转换为响应格式（列表接口不返回明细项，items 恒空）
        let transfer_details: Vec<InventoryTransferDetail> = views
            .into_iter()
            .map(|v| InventoryTransferDetail {
                id: v.id,
                transfer_no: v.transfer_no,
                from_warehouse_id: v.from_warehouse_id,
                to_warehouse_id: v.to_warehouse_id,
                transfer_date: v.transfer_date,
                status: v.status,
                total_quantity: v.total_quantity,
                total_amount: v.total_amount,
                notes: v.notes,
                created_by: v.created_by,
                approved_by: v.approved_by,
                approved_at: v.approved_at,
                shipped_at: v.shipped_at,
                received_at: v.received_at,
                created_at: v.created_at,
                updated_at: v.updated_at,
                from_warehouse_name: v.from_warehouse_name,
                to_warehouse_name: v.to_warehouse_name,
                created_by_name: v.created_by_name,
                items: vec![], // 列表接口不返回明细项
            })
            .collect();

        Ok(PaginatedResponse::new(
            transfer_details,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取库存调拨详情（包含明细项）
    /// 新增 data_scope 参数对单资源做 IDOR 校验；本表无 department_id 列，Dept 按归属人 ∈
    /// 可见部门成员集合判定（owner 取 `created_by: Option<i32>`）。
    pub async fn get_transfer_detail(
        &self,
        transfer_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 表头：与列表同构的单次 JOIN 富化查询（from/to 两仓库分别别名 JOIN + created_by 用户名）。
        let created_by_rel: RelationDef = InventoryTransferEntity::belongs_to(user::Entity)
            .from(inventory_transfer::Column::CreatedBy)
            .to(user::Column::Id)
            .into();
        let header = InventoryTransferEntity::find()
            .expr_as(
                Expr::col((Alias::new("from_wh"), warehouse::Column::Name)),
                "from_warehouse_name",
            )
            .expr_as(
                Expr::col((Alias::new("to_wh"), warehouse::Column::Name)),
                "to_warehouse_name",
            )
            .column_as(user::Column::RealName, "created_by_name")
            .join_as(
                JoinType::LeftJoin,
                inventory_transfer::Relation::FromWarehouse.def(),
                "from_wh",
            )
            .join_as(
                JoinType::LeftJoin,
                inventory_transfer::Relation::ToWarehouse.def(),
                "to_wh",
            )
            .join(JoinType::LeftJoin, created_by_rel)
            .filter(inventory_transfer::Column::Id.eq(transfer_id))
            .into_model::<InventoryTransferView>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 行级数据权限 IDOR 校验（created_by 由视图携带）
        // inventory_transfer 表无 department_id 列，归属判定走成员集合语义
        // （与列表侧 apply_data_scope Dept 分支同源），created_by 为 Option<i32>。
        if let Some(ctx) = data_scope {
            if !check_resource_owner_by_member_scope(ctx, header.created_by) {
                return Err(AppError::permission_denied(
                    "无权访问库存调拨单（数据范围限制）".to_string(),
                ));
            }
        }

        // 明细项：单次 LEFT JOIN products 富化取 product_code/product_name/grade/unit（无逐行回查）
        let item_details: Vec<InventoryTransferItemDetail> = InventoryTransferItemEntity::find()
            .column_as(product::Column::Code, "product_code")
            .column_as(product::Column::Name, "product_name")
            .column_as(product::Column::ProductGrade, "grade")
            .column_as(product::Column::Unit, "unit")
            .join(
                JoinType::LeftJoin,
                inventory_transfer_item::Relation::Product.def(),
            )
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .order_by(inventory_transfer_item::Column::Id, Order::Asc)
            .into_model::<InventoryTransferItemDetail>()
            .all(&*self.db)
            .await?;

        Ok(InventoryTransferDetail {
            id: header.id,
            transfer_no: header.transfer_no,
            from_warehouse_id: header.from_warehouse_id,
            to_warehouse_id: header.to_warehouse_id,
            transfer_date: header.transfer_date,
            status: header.status,
            total_quantity: header.total_quantity,
            total_amount: header.total_amount,
            notes: header.notes,
            created_by: header.created_by,
            approved_by: header.approved_by,
            approved_at: header.approved_at,
            shipped_at: header.shipped_at,
            received_at: header.received_at,
            created_at: header.created_at,
            updated_at: header.updated_at,
            from_warehouse_name: header.from_warehouse_name,
            to_warehouse_name: header.to_warehouse_name,
            created_by_name: header.created_by_name,
            items: item_details,
        })
    }

    /// 校验调拨主单状态取值是否落在权威词表内（`status::inventory_transfer::ALL`）。
    ///
    /// 词表唯一来源是写入方常量模块（本仓口径：`models/status/purchase_inventory.rs`），
    /// 本函数只做"取值域"判定，不判定流转是否合法（那由 `validate_transfer_status_write`
    /// 与本文件/`batch.rs` 列出的权威落点负责）。越界值一律拒绝，绝不"识别不了就保持原值"
    /// 或"识别不了就写进去"：静默吞掉是脏数据的入口（前端 `normalizeInventoryTransferStatus`
    /// 对同一取值域是抛错口径，两端必须一致）。
    fn validate_transfer_status_token(raw: &str) -> Result<(), AppError> {
        if !transfer_status::ALL.contains(&raw) {
            // 越界原值只进日志（可能含任意用户输入），响应文案只给公开取值域
            tracing::warn!(
                target: "inventory_transfer",
                rule = "status_token_domain",
                raw_status = raw,
                "调拨状态取值不在权威词表内，拒绝写入"
            );
            return Err(AppError::business_displayable(format!(
                "调拨状态取值无效（允许的调拨状态：{}）",
                transfer_status::ALL.join("、")
            )));
        }
        Ok(())
    }

    /// 建单口的状态门：新建调拨单的初始状态唯一 = 待审批。
    ///
    /// 依据（同树三处，全部是"写入方即权威"的事实而非注释）：
    /// - `build_transfer_active_model` 的缺省值就是 `transfer_status::PENDING`；
    /// - DB 列默认值同为 pending（migration `domain/v15/mod.rs:4381`
    ///   `ALTER COLUMN "status" SET DEFAULT 'pending'`）；
    /// - 审批/发货/收货各自的前置状态比较（本文件 `approve_transfer` 只接受 pending、
    ///   `batch.rs:120` 只接受 approved、`batch.rs:608` 只接受 shipped）意味着
    ///   一条以其它状态"出生"的单据会永久卡在流程外：既进不了审批，也进不了发货/收货，
    ///   却能被库存操作误读——所以建单不接受任何非初始状态。
    ///
    /// 状态推进的唯一途径是下面的权威操作函数，不是建单参数。
    fn validate_transfer_initial_status(target: &str) -> Result<(), AppError> {
        Self::validate_transfer_status_token(target)?;
        if target != transfer_status::PENDING {
            tracing::warn!(
                target: "inventory_transfer",
                rule = "initial_status_only_pending",
                requested_status = target,
                "建单请求要求以非初始状态落库，拒绝"
            );
            return Err(AppError::business_displayable(
                "新建调拨单的初始状态只能是待审批，建单后请通过审批、发货、收货操作推进状态",
            ));
        }
        Ok(())
    }

    /// 编辑保存口（PUT）的状态门：状态是状态机位，不是可编辑字段。
    ///
    /// 本入口的合法流转集合为空——不是"忘了配白名单"，而是本表全部真流转都绑在
    /// 带副作用的权威操作上，只写一列状态无法履行它们：
    /// - `pending → approved / rejected`：`approve_transfer`（本文件）额外做分级审批
    ///   角色校验（`inventory_transfer::can_approve_at_level`）并落
    ///   approved_by/approved_at/approval_level/approved_by_role；
    /// - `approved → shipped`：`ship_transfer`（`batch.rs:86`）额外扣减调出仓库存、
    ///   写出库流水、维护在途与 shipped_at 并发布事件；
    /// - `shipped → completed`：`receive_transfer`（`batch.rs:544`）额外做调入仓入库、
    ///   明细收货量与 received_at 并发布事件。
    ///
    /// 因此绕过这些函数改状态必然造成"状态说已发货、库存一分没动"的账实分裂
    /// （或让低权限角色借普通编辑拿到审批结果）。异值一律拒绝；与当前状态逐字符相同
    /// 的幂等写入放行（那才是"没改状态"），键缺席同样不改（见 `apply_transfer_main_update`
    /// 的三态写入）。
    fn validate_transfer_status_write(current: &str, target: &str) -> Result<(), AppError> {
        Self::validate_transfer_status_token(target)?;
        if target != current {
            tracing::warn!(
                target: "inventory_transfer",
                rule = "status_transition_authority_only",
                current_status = current,
                target_status = target,
                "编辑保存口试图驱动状态流转，拒绝（流转走审批/发货/收货权威操作）"
            );
            return Err(AppError::business_displayable(
                "调拨单状态只能通过审批、发货、收货操作变更，编辑保存不改变状态",
            ));
        }
        Ok(())
    }

    /// 校验调拨单号唯一性，冲突时回滚事务
    async fn validate_transfer_no_uniqueness(
        txn: &DatabaseTransaction,
        transfer_no: &str,
    ) -> Result<(), AppError> {
        let existing = InventoryTransferEntity::find()
            .filter(inventory_transfer::Column::TransferNo.eq(transfer_no))
            .one(txn)
            .await?;
        if existing.is_some() {
            tracing::error!("调拨单号 {} 已存在，事务将自动回滚", transfer_no);
            return Err(AppError::business("调拨单号已存在，请重试".to_string()));
        }
        Ok(())
    }

    /// 构建调拨主表 ActiveModel（状态默认 PENDING，数量与金额初始为 0，审批层级 L1）
    /// created_by 落当前操作人：Self/Dept 数据范围过滤与 created_by_name 富化均依赖此列。
    fn build_transfer_active_model(
        transfer_no: String,
        from_warehouse_id: i32,
        to_warehouse_id: i32,
        transfer_date: Option<chrono::DateTime<chrono::Utc>>,
        status: Option<String>,
        notes: Option<String>,
        user_id: i32,
    ) -> inventory_transfer::ActiveModel {
        inventory_transfer::ActiveModel {
            id: Default::default(),
            transfer_no: sea_orm::ActiveValue::Set(transfer_no),
            from_warehouse_id: sea_orm::ActiveValue::Set(from_warehouse_id),
            to_warehouse_id: sea_orm::ActiveValue::Set(to_warehouse_id),
            transfer_date: sea_orm::ActiveValue::Set(
                transfer_date.unwrap_or_else(chrono::Utc::now),
            ),
            status: sea_orm::ActiveValue::Set(
                status.unwrap_or_else(|| transfer_status::PENDING.to_string()),
            ),
            total_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            notes: sea_orm::ActiveValue::Set(notes),
            // inventory_transfer.created_by 为 Option<i32>（可空列），建单时如实落当前操作人
            created_by: sea_orm::ActiveValue::Set(Some(user_id)),
            approved_by: sea_orm::ActiveValue::NotSet,
            approved_at: sea_orm::ActiveValue::NotSet,
            shipped_at: sea_orm::ActiveValue::NotSet,
            received_at: sea_orm::ActiveValue::NotSet,
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            // P1 batch-18 缺陷 6.1：调拨分级审批字段（初始 L1，待明细创建后按总金额更新）
            approval_level: sea_orm::ActiveValue::Set(Some(
                inventory_transfer::APPROVAL_LEVEL_L1.to_string(),
            )),
            approved_by_role: sea_orm::ActiveValue::NotSet,
            total_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
        }
    }

    /// 批量创建调拨明细项并累计总数量与总金额
    async fn create_transfer_items_and_compute_total(
        txn: &DatabaseTransaction,
        transfer_id: i32,
        items: Vec<InventoryTransferItemRequest>,
    ) -> Result<(rust_decimal::Decimal, rust_decimal::Decimal), AppError> {
        let mut total_quantity = rust_decimal::Decimal::ZERO;
        let mut total_amount = rust_decimal::Decimal::ZERO;
        for item_req in items {
            // 调拨单明细即出库方向：白坯/染色判定与缸号/批次/匹号必填统一走 fabric_class
            // 单一实现（经 require_outbound_dimensions 包装，用户 2026-10-02 口径：
            // 出库对染色布强制四维=缸号/色号/批次/匹号；仅以是否为空判定，不看名称）。
            let product_id = item_req
                .product_id
                .ok_or_else(|| AppError::validation_displayable("调拨明细缺少物料ID"))?;
            let dims = require_outbound_dimensions(
                "调拨出库明细",
                product_id,
                item_req.color_no.as_deref(),
                item_req.dye_lot_no.as_deref(),
                item_req.batch_no.as_deref(),
                item_req.piece_no.as_deref(),
            )?;

            let quantity = item_req.quantity.unwrap_or(rust_decimal::Decimal::ZERO);
            total_quantity += quantity;
            // P1 batch-18 缺陷 6.1：累计总金额（quantity × unit_cost）
            if let Some(uc) = item_req.unit_cost {
                total_amount += quantity * uc;
            }
            let item = inventory_transfer_item::ActiveModel {
                id: Default::default(),
                transfer_id: sea_orm::ActiveValue::Set(transfer_id),
                product_id: sea_orm::ActiveValue::Set(product_id),
                quantity: sea_orm::ActiveValue::Set(quantity),
                shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                received_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                // P1 batch-18 缺陷 6.1：unit_cost 从请求传入（用于计算总金额）
                unit_cost: sea_orm::ActiveValue::Set(item_req.unit_cost),
                notes: sea_orm::ActiveValue::Set(item_req.notes),
                created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                // 面料追溯字段：白坯/染色校验归一后如实写入（禁止 NotSet 丢弃入参值）
                color_no: sea_orm::ActiveValue::Set(dims.color_no),
                // 列 DDL 为 NOT NULL DEFAULT ''（system/mod.rs:292）：白坯布缸号合法缺省，
                // 用 NotSet 让 DB DEFAULT '' 生效，Set(None) 会触发 NOT NULL 违例
                dye_lot_no: dims
                    .dye_lot_no
                    .map(|v| sea_orm::ActiveValue::Set(Some(v)))
                    .unwrap_or(sea_orm::ActiveValue::NotSet),
                batch_no: sea_orm::ActiveValue::Set(dims.batch_no),
                // 匹号列为 m0066 补的可空列（DB 默认 NULL）：白坯 Set(None) 如实落 NULL
                piece_no: sea_orm::ActiveValue::Set(dims.piece_no),
            };
            item.insert(txn).await?;
        }
        Ok((total_quantity, total_amount))
    }

    /// 更新调拨单总数量并写入审计日志
    async fn save_transfer_total_quantity(
        txn: &DatabaseTransaction,
        transfer_id: i32,
        total_quantity: rust_decimal::Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        let transfer_entity = InventoryTransferEntity::find_by_id(transfer_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::business("调拨单不存在"))?;
        let mut transfer_update: inventory_transfer::ActiveModel = transfer_entity.into();
        transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            transfer_update,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 创建库存调拨
    pub async fn create_transfer(
        &self,
        request: CreateInventoryTransferRequest,
        // 操作人取会话 user_id，供审计日志留痕
        user_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 建单状态门在任何 DB 访问前：status 键缺席时由
        // `build_transfer_active_model` 落缺省 pending（合法），送来显式值才判。
        if let Some(raw) = request.status.as_deref() {
            Self::validate_transfer_initial_status(raw)?;
        }
        let txn = (*self.db).begin().await?;
        let transfer_no = self.generate_transfer_no().await?;
        Self::validate_transfer_no_uniqueness(&txn, &transfer_no).await?;
        let from_warehouse_id = request
            .from_warehouse_id
            .ok_or_else(|| AppError::validation_displayable("调拨单缺少调出仓库ID"))?;
        let to_warehouse_id = request
            .to_warehouse_id
            .ok_or_else(|| AppError::validation_displayable("调拨单缺少调入仓库ID"))?;
        let items = request.items.unwrap_or_default();
        let transfer = Self::build_transfer_active_model(
            transfer_no,
            from_warehouse_id,
            to_warehouse_id,
            request.transfer_date,
            request.status,
            request.notes,
            user_id,
        );
        let transfer_entity = transfer.insert(&txn).await?;
        let transfer_id = transfer_entity.id;
        self.check_from_warehouse_inventory(&from_warehouse_id, &items, &txn)
            .await?;
        let (total_quantity, _total_amount) =
            Self::create_transfer_items_and_compute_total(&txn, transfer_id, items).await?;
        Self::save_transfer_total_quantity(&txn, transfer_id, total_quantity, user_id).await?;
        txn.commit().await?;
        self.get_transfer_detail(transfer_id, None).await
    }

    /// 加载调拨单并校验状态非已完成
    async fn load_transfer_for_update(
        &self,
        transfer_id: i32,
    ) -> Result<inventory_transfer::Model, AppError> {
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;
        if transfer.status == transfer_status::COMPLETED {
            // 状态门拒绝文案是用户可理解的公开业务规则（已完成单据不可编辑、状态须走
            // 审批/发货/收货权威操作），不含数据库记录 ID、不含 SQL/表名/他人数据，满足
            // `utils/error.rs` 模块文档的可外显安全边界 ⇒ 走 `business_displayable`，
            // 与相邻状态门 `Self::validate_transfer_status_write` 口径一致；
            // 若仍走脱敏 `business`，出参被替换成 `err_msg::BUSINESS_PUBLIC`，用户看不到拒绝原因。
            return Err(AppError::business_displayable(
                "调拨单已完成，不允许修改：调拨单状态只能通过审批、发货、收货操作变更，编辑保存不改变状态",
            ));
        }
        Ok(transfer)
    }

    /// 应用主表字段更新（status/notes/updated_at）并写审计日志
    ///
    /// status 分支是**编辑保存口的唯一状态写落点**，故状态门落在这里而不是只落调用方：
    /// 新增调用方无法绕过（见 `validate_transfer_status_write` 的权威落点清单）。
    async fn apply_transfer_main_update(
        txn: &DatabaseTransaction,
        transfer: inventory_transfer::Model,
        status: Option<Option<String>>,
        notes: Option<Option<String>>,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 三态写入：None=不 Set、Some(None)=Set(None) 置 NULL、Some(Some(v))=Set(v) 覆盖。
        // status 对应实体 Model 非 Option 列（Some(None) 已在 update_transfer 入口拒绝）：
        // 仅覆盖/保持，且"覆盖"只允许覆盖成同一值——真流转归权威操作函数。
        if let Some(target) = status.as_ref().and_then(|inner| inner.as_deref()) {
            // 门必须打在**事务内加锁读到的当前值**上：入参 model 来自事务外的快照读，
            // 该行可能已被审批/发货/收货推进（那三个函数都用 lock_exclusive 串行化）。
            // 若拿过期快照判"同值幂等"，就会把已被推进的行改写回旧状态——等价于绕过状态机。
            let locked = InventoryTransferEntity::find_by_id(transfer.id)
                .lock_exclusive()
                .one(txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer.id)))?;
            Self::validate_transfer_status_write(&locked.status, target)?;
        }
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        if let Some(status) = status.flatten() {
            transfer_update.status = sea_orm::ActiveValue::Set(status);
        }
        // notes 为 DB 可空列：Some(inner)=Set(inner)，显式 null 直落 NULL
        if let Some(notes) = notes {
            transfer_update.notes = sea_orm::ActiveValue::Set(notes);
        }
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            transfer_update,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 删除调拨单原有明细项
    async fn clear_transfer_items(
        txn: &DatabaseTransaction,
        transfer_id: i32,
    ) -> Result<(), AppError> {
        InventoryTransferItemEntity::delete_many()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .exec(txn)
            .await?;
        Ok(())
    }

    /// 重建调拨明细项并累计总数量与总金额
    /// P1 batch-18 缺陷 6.2：与 create_transfer_items_and_compute_total 保持一致的缸号校验
    async fn rebuild_transfer_items_and_total(
        txn: &DatabaseTransaction,
        transfer_id: i32,
        items: Vec<InventoryTransferItemRequest>,
    ) -> Result<(rust_decimal::Decimal, rust_decimal::Decimal), AppError> {
        let mut total_quantity = rust_decimal::Decimal::ZERO;
        let mut total_amount = rust_decimal::Decimal::ZERO;
        for item_req in items {
            // 调拨单明细即出库方向（重建路径与建单路径同口径，防"改一处未推广"）：
            // 判定唯一来源 fabric_class 经 require_outbound_dimensions 包装——
            // 用户 2026-10-02 口径：出库对染色布强制四维=缸号/色号/批次/匹号。
            let product_id = item_req
                .product_id
                .ok_or_else(|| AppError::validation_displayable("调拨明细缺少物料ID"))?;
            let dims = require_outbound_dimensions(
                "调拨出库明细",
                product_id,
                item_req.color_no.as_deref(),
                item_req.dye_lot_no.as_deref(),
                item_req.batch_no.as_deref(),
                item_req.piece_no.as_deref(),
            )?;

            let quantity = item_req.quantity.unwrap_or(rust_decimal::Decimal::ZERO);
            total_quantity += quantity;
            // P1 batch-18 缺陷 6.1：累计总金额（quantity × unit_cost）
            if let Some(uc) = item_req.unit_cost {
                total_amount += quantity * uc;
            }
            let item = inventory_transfer_item::ActiveModel {
                id: Default::default(),
                transfer_id: sea_orm::ActiveValue::Set(transfer_id),
                product_id: sea_orm::ActiveValue::Set(product_id),
                quantity: sea_orm::ActiveValue::Set(quantity),
                shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                received_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                // P1 batch-18 缺陷 6.1：unit_cost 从请求传入（用于计算总金额）
                unit_cost: sea_orm::ActiveValue::Set(item_req.unit_cost),
                notes: sea_orm::ActiveValue::Set(item_req.notes),
                created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                // 面料追溯字段：白坯/染色校验归一后如实写入（禁止 NotSet 丢弃入参值）
                color_no: sea_orm::ActiveValue::Set(dims.color_no),
                // 列 DDL 为 NOT NULL DEFAULT ''（system/mod.rs:292）：白坯缸号用 NotSet 落 DEFAULT ''
                dye_lot_no: dims
                    .dye_lot_no
                    .map(|v| sea_orm::ActiveValue::Set(Some(v)))
                    .unwrap_or(sea_orm::ActiveValue::NotSet),
                batch_no: sea_orm::ActiveValue::Set(dims.batch_no),
                // 匹号列为 m0066 补的可空列：白坯 Set(None) 如实落 NULL
                piece_no: sea_orm::ActiveValue::Set(dims.piece_no),
            };
            item.insert(txn).await?;
        }
        Ok((total_quantity, total_amount))
    }

    /// 更新调拨单总数量与总金额并写审计日志
    /// P1 batch-18 缺陷 6.1：更新路径同步刷新 total_amount 以支持分级审批
    async fn save_transfer_total_update(
        txn: &DatabaseTransaction,
        transfer_id: i32,
        total_quantity: rust_decimal::Decimal,
        total_amount: rust_decimal::Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        let transfer_entity = InventoryTransferEntity::find_by_id(transfer_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::business("调拨单不存在"))?;
        let mut transfer_update: inventory_transfer::ActiveModel = transfer_entity.into();
        transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
        // P1 batch-18 缺陷 6.1：同步更新 total_amount 以确保后续分级审批基于最新金额
        transfer_update.total_amount = sea_orm::ActiveValue::Set(total_amount);
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            transfer_update,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 更新库存调拨
    pub async fn update_transfer(
        &self,
        transfer_id: i32,
        request: UpdateInventoryTransferRequest,
        user_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // NOT NULL 列门控（status 对应实体 Model 非 Option 列）：显式 null 是调用方错误
        // 而非"保持原值"，在任何 DB 访问前拒绝，错误外显不脱敏
        if matches!(request.status, Some(None)) {
            return Err(AppError::business_displayable(
                "调拨状态不能清空：该字段为必填项",
            ));
        }
        let transfer = self.load_transfer_for_update(transfer_id).await?;
        // 状态门（词表域 + "编辑保存不驱动流转"）落在 `apply_transfer_main_update`
        // ——本入口唯一的状态写落点，随状态一起进入事务：门不过 ⇒ 事务回滚，
        // notes/items 也不会出现"改了一半"。
        let txn = (*self.db).begin().await?;
        Self::apply_transfer_main_update(
            &txn,
            transfer,
            request.status.clone(),
            request.notes.clone(),
            user_id,
        )
        .await?;
        if let Some(items) = request.items {
            Self::clear_transfer_items(&txn, transfer_id).await?;
            let (total_quantity, total_amount) =
                Self::rebuild_transfer_items_and_total(&txn, transfer_id, items).await?;
            Self::save_transfer_total_update(
                &txn,
                transfer_id,
                total_quantity,
                total_amount,
                user_id,
            )
            .await?;
        }
        txn.commit().await?;
        self.get_transfer_detail(transfer_id, None).await
    }

    /// 审核库存调拨
    ///
    /// P1 batch-18 缺陷 6.1：接入分级审批逻辑
    /// - 根据调拨总金额计算审批层级（L1<1万 / L2 1-10万 / L3>10万）
    /// - 校验审批人角色是否具备该层级审批权限
    /// - 持久化 approval_level 与 approved_by_role 字段
    pub async fn approve_transfer(
        &self,
        transfer_id: i32,
        approved: bool,
        notes: Option<String>,
        // 操作人取会话 user_id，供审计日志留痕
        user_id: i32,
        // P1 batch-18 缺陷 6.1：注入审批人 role_id 用于分级审批权限校验
        role_id: Option<i32>,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 检查调拨单是否存在（行锁，串行化并发状态变更）
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有待审核的调拨单可以审核
        if transfer.status != transfer_status::PENDING {
            // 同族状态门：拒绝文案「只有待审核状态的调拨单可以审核」是静态公开业务规则，
            // 不含记录 ID/查询所得实体数据/权限判定依据，满足 `utils/error.rs` 的可外显安全边界
            // ⇒ 走 `business_displayable`，与建单/编辑/完成各状态门口径一致，让用户看到拒绝原因。
            return Err(AppError::business_displayable(
                "只有待审核状态的调拨单可以审核",
            ));
        }

        // P1 batch-18 缺陷 6.1：根据调拨总金额计算审批层级
        let required_level = inventory_transfer::determine_approval_level(transfer.total_amount);

        // P1 batch-18 缺陷 6.1：审批通过时校验审批人角色权限
        let approver_role_code: Option<String> = if approved {
            let rid = role_id.ok_or_else(|| {
                AppError::permission_denied("缺陷 6.1：审批人角色 ID 缺失，无法校验分级审批权限")
            })?;
            let role_code = crate::utils::admin_checker::get_role_code(&*self.db, rid)
                .await
                .ok_or_else(|| {
                    AppError::permission_denied("缺陷 6.1：无法查询审批人角色信息，拒绝审批")
                })?;
            if !inventory_transfer::can_approve_at_level(&role_code, required_level) {
                return Err(AppError::permission_denied(format!(
                    "缺陷 6.1：调拨金额 {} 元需 {} 级审批，当前角色 {} 无权审批",
                    transfer.total_amount, required_level, role_code
                )));
            }
            Some(role_code)
        } else {
            // 驳回场景不强制校验角色权限，但仍记录角色信息（若有）
            if let Some(rid) = role_id {
                crate::utils::admin_checker::get_role_code(&*self.db, rid).await
            } else {
                None
            }
        };

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        if approved {
            transfer_update.status =
                sea_orm::ActiveValue::Set(transfer_status::APPROVED.to_string());
            // P1 batch-18 缺陷 6.1：修复 approved_by 字段未持久化的 bug（原 NotSet 导致审批人丢失）
            transfer_update.approved_by = sea_orm::ActiveValue::Set(Some(user_id));
            transfer_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        } else {
            transfer_update.status =
                sea_orm::ActiveValue::Set(transfer_status::REJECTED.to_string());
        }
        // P1 batch-18 缺陷 6.1：持久化分级审批字段（审批层级与审批人角色）
        transfer_update.approval_level =
            sea_orm::ActiveValue::Set(Some(required_level.to_string()));
        transfer_update.approved_by_role = sea_orm::ActiveValue::Set(approver_role_code);
        if let Some(n) = notes {
            transfer_update.notes = sea_orm::ActiveValue::Set(Some(n));
        }
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            // 操作人取会话 user_id（不再用 0 占位），便于审计追踪
            Some(user_id),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id, None).await
    }

    // 生成调拨单号
    crate::impl_generate_no!(
        generate_transfer_no,
        "TRF",
        inventory_transfer::Entity,
        inventory_transfer::Column::TransferNo
    );

    /// 删除调拨单（仅 pending/rejected 状态）
    pub async fn delete_transfer(
        &self,
        transfer_id: i32,
        // 操作人取会话 user_id，供审计日志留痕
        user_id: i32,
    ) -> Result<(), AppError> {
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        if transfer.status == transfer_status::APPROVED
            || transfer.status == transfer_status::SHIPPED
            || transfer.status == transfer_status::COMPLETED
        {
            // 保持脱敏 `business`：文案经 `format!` 塞入服务端查得的 `transfer.status`（实体数据），
            // 触到 `utils/error.rs`「禁止把查询到的实体数据拼进可外显文案」硬边界，
            // 与建单/审批/完成门（静态规则文案）不同族 ⇒ 不随本批收口为 displayable。
            return Err(AppError::business(format!(
                "调拨单状态 {} 不允许删除",
                transfer.status
            )));
        }

        let txn = (*self.db).begin().await?;
        InventoryTransferItemEntity::delete_many()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .exec(&txn)
            .await?;
        // P0 8-3 修复：delete 操作补审计日志
        // 操作人取会话 user_id（不再用 0 占位），便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            InventoryTransferEntity,
            _,
        >(&txn, "inventory_transfer", transfer_id, Some(user_id))
        .await?;
        txn.commit().await?;
        Ok(())
    }
}
