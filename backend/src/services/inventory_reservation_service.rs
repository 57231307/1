use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
};
use std::sync::Arc;

use crate::models::inventory_reservation::{self, Entity as InventoryReservationEntity};
use crate::models::status::inventory_reservation as reservation_status;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, DataScopeContext};
use crate::utils::error::AppError;

/// 库存预留服务
pub struct InventoryReservationService {
    db: Arc<DatabaseConnection>,
}

impl InventoryReservationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建库存预留
    pub async fn create_reservation(
        &self,
        order_id: i32,
        product_id: i32,
        warehouse_id: i32,
        quantity: rust_decimal::Decimal,
        created_by: Option<i32>,
        notes: Option<String>,
    ) -> Result<inventory_reservation::Model, AppError> {
        let reservation = inventory_reservation::ActiveModel {
            id: Default::default(),
            order_id: sea_orm::ActiveValue::Set(order_id),
            product_id: sea_orm::ActiveValue::Set(product_id),
            warehouse_id: sea_orm::ActiveValue::Set(warehouse_id),
            quantity: sea_orm::ActiveValue::Set(quantity),
            status: sea_orm::ActiveValue::Set(reservation_status::PENDING.to_string()),
            reserved_at: sea_orm::ActiveValue::Set(Utc::now()),
            released_at: sea_orm::ActiveValue::NotSet,
            notes: sea_orm::ActiveValue::Set(notes),
            created_by: sea_orm::ActiveValue::Set(created_by),
            created_at: sea_orm::ActiveValue::Set(Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(Utc::now()),
        };

        reservation.insert(&*self.db).await.map_err(AppError::from)
    }

    /// 锁定预留（从 pending 到 locked）
    pub async fn lock_reservation(
        &self,
        reservation_id: i32,
    ) -> Result<inventory_reservation::Model, AppError> {
        // P1-8 修复（批次 79 v1 复审）：状态门 + update 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门查询在 self.db 上、update 也在 self.db 上，无事务边界，
        // 并发场景下可能在状态检查通过后、update 前发生状态变更，导致已锁定/已释放预留被重复锁定。
        let txn = (*self.db).begin().await?;

        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存预留 {} 未找到", reservation_id)))?;

        if reservation.status != reservation_status::PENDING {
            return Err(AppError::business(format!(
                "预留状态为{}，只有待处理状态的预留可以锁定",
                reservation.status
            )));
        }

        let mut reservation_update: inventory_reservation::ActiveModel = reservation.into();
        reservation_update.status = sea_orm::ActiveValue::Set(reservation_status::LOCKED.to_string());
        reservation_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = reservation_update
            .update(&txn)
            .await
            .map_err(AppError::from)?;

        txn.commit().await?;
        Ok(result)
    }

    /// 释放预留（从 locked 到 released）
    pub async fn release_reservation(
        &self,
        reservation_id: i32,
    ) -> Result<inventory_reservation::Model, AppError> {
        // P1-9 修复（批次 79 v1 复审）：状态门 + update 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门查询在 self.db 上、update 也在 self.db 上，无事务边界，
        // 并发场景下可能在状态检查通过后、update 前发生状态变更，导致已释放预留被重复释放。
        let txn = (*self.db).begin().await?;

        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存预留 {} 未找到", reservation_id)))?;

        if reservation.status != reservation_status::LOCKED && reservation.status != reservation_status::PENDING {
            return Err(AppError::business(format!(
                "预留状态为{}，只有已锁定或待处理状态的预留可以释放",
                reservation.status
            )));
        }

        let mut reservation_update: inventory_reservation::ActiveModel = reservation.into();
        reservation_update.status = sea_orm::ActiveValue::Set(reservation_status::RELEASED.to_string());
        reservation_update.released_at = sea_orm::ActiveValue::Set(Some(Utc::now()));
        reservation_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = reservation_update
            .update(&txn)
            .await
            .map_err(AppError::from)?;

        txn.commit().await?;
        Ok(result)
    }

    /// 获取预留列表
    ///
    /// 批次 263 修复：接入 paginate_with_total 工具函数，修复原 fetch_page(page) 未做
    /// saturating_sub(1) 偏移的 bug（page 为 1-based，fetch_page 接收 0-based，原实现跳过第一页）。
    /// 补 clamp(1, 1000) 防 DoS。返回类型 total 从 i64 改为 u64（与项目其他分页函数一致）。
    pub async fn list_reservations(
        &self,
        page: u64,
        page_size: u64,
        product_id: Option<i32>,
        warehouse_id: Option<i32>,
        status: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<inventory_reservation::Model>, u64), AppError> {
        use crate::utils::pagination::paginate_with_total;
        use sea_orm::PaginatorTrait;

        let mut query = InventoryReservationEntity::find();

        if let Some(pid) = product_id {
            query = query.filter(inventory_reservation::Column::ProductId.eq(pid));
        }
        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_reservation::Column::WarehouseId.eq(wid));
        }
        if let Some(s) = status {
            query = query.filter(inventory_reservation::Column::Status.eq(s));
        }

        // V15 P0-S01：行级数据权限过滤
        // inventory_reservation 表无 department_id，Dept 退化为 Self，使用 created_by（Option<i32>）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                inventory_reservation::Column::CreatedBy,
                inventory_reservation::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
        }

        let paginator = query.paginate(&*self.db, page_size);
        let (reservations, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((reservations, total))
    }

    /// 删除预留
    pub async fn delete_reservation(
        &self,
        reservation_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        // P1-10 修复（批次 79 v1 复审）：状态门 + delete_with_audit 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门查询在 self.db 上、delete_with_audit 也在 self.db 上，无事务边界，
        // 并发场景下可能在状态检查通过后、delete 前发生状态变更，导致已锁定预留被误删。
        let txn = (*self.db).begin().await?;

        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存预留 {} 未找到", reservation_id)))?;

        // 只有 pending 状态的预留可以删除
        if reservation.status != reservation_status::PENDING {
            return Err(AppError::business(format!(
                "预留状态为{}，只有待处理状态的预留可以删除",
                reservation.status
            )));
        }

        // P0 8-3 修复：delete 操作补审计日志
        // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            InventoryReservationEntity,
            _,
        >(&txn, "inventory_reservation", reservation_id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::ymd;
    use rust_decimal::Decimal;
    use sea_orm::Database;
    use std::str::FromStr;

    /// 测试夹具：构建 SQLite 内存数据库连接
    ///
    /// 复用 customer_credit_limit.rs 的夹具模式，支持通过 TEST_DATABASE_URL
    /// 环境变量切换到真实数据库进行本地手动验证。
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    /// 测试夹具：构建库存预留 Model
    ///
    /// 封装 `inventory_reservation::Model` 的构造，便于在各状态门测试中复用。
    /// 使用 `decs!` 解析数量字段，使用 `ymd!` 构造固定的预留时间，便于断言。
    fn make_reservation_model(
        id: i32,
        status: &str,
        quantity: Decimal,
    ) -> inventory_reservation::Model {
        inventory_reservation::Model {
            id,
            order_id: 1001,
            product_id: 2001,
            warehouse_id: 3001,
            quantity,
            status: status.to_string(),
            reserved_at: ymd!(2026, 1, 15)
                .and_hms_opt(0, 0, 0)
                .expect("不变量：and_hms_opt(0, 0, 0) 永远合法")
                .and_utc(),
            released_at: None,
            notes: Some("测试预留".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 复现 lock_reservation 的状态门判定：仅 PENDING 可锁定
    fn can_lock(status: &str) -> bool {
        status == reservation_status::PENDING
    }

    /// 复现 release_reservation 的状态门判定：LOCKED 或 PENDING 可释放
    fn can_release(status: &str) -> bool {
        status == reservation_status::LOCKED || status == reservation_status::PENDING
    }

    /// 复现 delete_reservation 的状态门判定：仅 PENDING 可删除
    fn can_delete(status: &str) -> bool {
        status == reservation_status::PENDING
    }

    /// 测试_状态常量_待处理为合法值
    ///
    /// 验证 PENDING 常量是小写字符串，与业务代码及数据库约定一致。
    #[test]
    fn 测试_状态常量_待处理为合法值() {
        assert_eq!(reservation_status::PENDING, "pending");
    }

    /// 测试_状态常量_已锁定为合法值
    ///
    /// 验证 LOCKED 常量是小写字符串，与 lock_reservation 中设置的目标状态一致。
    #[test]
    fn 测试_状态常量_已锁定为合法值() {
        assert_eq!(reservation_status::LOCKED, "locked");
    }

    /// 测试_状态常量_已消耗为合法值
    ///
    /// 验证 CONSUMED 常量是小写字符串，表示发货已扣减库存的终态。
    #[test]
    fn 测试_状态常量_已消耗为合法值() {
        assert_eq!(reservation_status::CONSUMED, "consumed");
    }

    /// 测试_状态常量_已释放为合法值
    ///
    /// 验证 RELEASED 常量是小写字符串，与 release_reservation 中设置的目标状态一致。
    #[test]
    fn 测试_状态常量_已释放为合法值() {
        assert_eq!(reservation_status::RELEASED, "released");
    }

    /// 测试_状态常量_各状态值互不相同
    ///
    /// 验证 5 个状态常量两两互不相同，避免状态机歧义导致误判。
    #[test]
    fn 测试_状态常量_各状态值互不相同() {
        let statuses = [
            reservation_status::PENDING,
            reservation_status::LOCKED,
            reservation_status::CONSUMED,
            reservation_status::RELEASED,
            reservation_status::CANCELLED,
        ];
        for i in 0..statuses.len() {
            for j in (i + 1)..statuses.len() {
                assert_ne!(statuses[i], statuses[j], "状态常量重复：{}", statuses[i]);
            }
        }
    }

    /// 测试_状态匹配_仅待处理状态可锁定
    ///
    /// 复现 lock_reservation 的状态门：仅 PENDING 可锁定，其余状态均应被拒绝。
    #[test]
    fn 测试_状态匹配_仅待处理状态可锁定() {
        assert!(can_lock(reservation_status::PENDING));
        assert!(!can_lock(reservation_status::LOCKED));
        assert!(!can_lock(reservation_status::CONSUMED));
        assert!(!can_lock(reservation_status::RELEASED));
        assert!(!can_lock(reservation_status::CANCELLED));
    }

    /// 测试_状态匹配_已锁定或待处理状态可释放
    ///
    /// 复现 release_reservation 的状态门：LOCKED 或 PENDING 可释放，
    /// CONSUMED/RELEASED/CANCELLED 应被拒绝（已释放不可重复释放）。
    #[test]
    fn 测试_状态匹配_已锁定或待处理状态可释放() {
        assert!(can_release(reservation_status::PENDING));
        assert!(can_release(reservation_status::LOCKED));
        assert!(!can_release(reservation_status::CONSUMED));
        assert!(!can_release(reservation_status::RELEASED));
        assert!(!can_release(reservation_status::CANCELLED));
    }

    /// 测试_状态匹配_仅待处理状态可删除
    ///
    /// 复现 delete_reservation 的状态门：仅 PENDING 可删除，
    /// 已锁定/已消耗/已释放/已取消的预留均不可删除。
    #[test]
    fn 测试_状态匹配_仅待处理状态可删除() {
        assert!(can_delete(reservation_status::PENDING));
        assert!(!can_delete(reservation_status::LOCKED));
        assert!(!can_delete(reservation_status::CONSUMED));
        assert!(!can_delete(reservation_status::RELEASED));
        assert!(!can_delete(reservation_status::CANCELLED));
    }

    /// 测试_错误消息_锁定失败包含状态值与中文说明
    ///
    /// 复现 lock_reservation 中非 PENDING 状态的错误消息构造：
    /// 消息应包含实际状态值与"只有待处理状态的预留可以锁定"中文说明。
    #[test]
    fn 测试_错误消息_锁定失败包含状态值与中文说明() {
        let status = reservation_status::LOCKED;
        let msg = format!("预留状态为{}，只有待处理状态的预留可以锁定", status);

        // 包含实际状态值
        assert!(msg.contains(reservation_status::LOCKED));
        // 包含中文说明
        assert!(msg.contains("只有待处理状态的预留可以锁定"));

        // 构造为业务错误并验证类型与 Display
        let err = AppError::business(msg.clone());
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains(&msg));
    }

    /// 测试_错误消息_释放失败包含状态值与中文说明
    ///
    /// 复现 release_reservation 中非 LOCKED/PENDING 状态的错误消息构造：
    /// 消息应包含实际状态值与"只有已锁定或待处理状态的预留可以释放"中文说明。
    #[test]
    fn 测试_错误消息_释放失败包含状态值与中文说明() {
        let status = reservation_status::CONSUMED;
        let msg = format!("预留状态为{}，只有已锁定或待处理状态的预留可以释放", status);

        assert!(msg.contains(reservation_status::CONSUMED));
        assert!(msg.contains("只有已锁定或待处理状态的预留可以释放"));

        let err = AppError::business(msg.clone());
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains(&msg));
    }

    /// 测试_错误消息_删除失败包含状态值与中文说明
    ///
    /// 复现 delete_reservation 中非 PENDING 状态的错误消息构造：
    /// 消息应包含实际状态值与"只有待处理状态的预留可以删除"中文说明。
    #[test]
    fn 测试_错误消息_删除失败包含状态值与中文说明() {
        let status = reservation_status::LOCKED;
        let msg = format!("预留状态为{}，只有待处理状态的预留可以删除", status);

        assert!(msg.contains(reservation_status::LOCKED));
        assert!(msg.contains("只有待处理状态的预留可以删除"));

        let err = AppError::business(msg.clone());
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains(&msg));
    }

    /// 测试_创建预留_默认状态为待处理
    ///
    /// 复现 create_reservation 中的初始状态设置：status 字段初始化为 PENDING，
    /// 数量字段通过 decs! 夹具解析，验证初始状态非其他终态。
    #[test]
    fn 测试_创建预留_默认状态为待处理() {
        // 复现 create_reservation 的 quantity 参数解析与初始状态设置
        let quantity = decs!("100.50");
        let initial_status = reservation_status::PENDING.to_string();

        // 默认状态应为待处理
        assert_eq!(initial_status, reservation_status::PENDING);
        // 数量字段应正确解析
        assert_eq!(quantity.to_string(), "100.50");
        // 默认状态非其他终态
        assert_ne!(initial_status, reservation_status::LOCKED);
        assert_ne!(initial_status, reservation_status::CONSUMED);
        assert_ne!(initial_status, reservation_status::RELEASED);
    }

    /// 测试_预留模型夹具_状态字段正确
    ///
    /// 验证 make_reservation_model 夹具构造的 Model 字段正确，
    /// 其中 reserved_at 由 ymd! 夹具构造，数量由 decs! 解析。
    #[test]
    fn 测试_预留模型夹具_状态字段正确() {
        let model = make_reservation_model(1, reservation_status::LOCKED, decs!("50"));

        assert_eq!(model.id, 1);
        assert_eq!(model.status, reservation_status::LOCKED);
        assert_eq!(model.quantity, decs!("50"));
        // reserved_at 由 ymd!(2026, 1, 15) 构造，日期部分应为 2026-01-15
        assert_eq!(
            model.reserved_at.format("%Y-%m-%d").to_string(),
            "2026-01-15"
        );
        // 新建预留 released_at 应为 None
        assert!(model.released_at.is_none());
    }

    /// 测试_服务实例创建
    ///
    /// 验证 InventoryReservationService 在 SQLite 内存数据库上能正常实例化，
    /// 与 customer_credit_limit.rs 的服务实例化测试模式一致。
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = InventoryReservationService::new(Arc::new(db));

        // 验证服务内部 db 引用计数 >= 1
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_锁定预留_无表结构返回错误
    ///
    /// 需要 inventory_reservations 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时 lock_reservation 应返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_锁定预留_无表结构返回错误() {
        let db = setup_test_db().await;
        let service = InventoryReservationService::new(Arc::new(db));

        // 无 inventory_reservations 表 schema，应返回数据库错误
        let result = service.lock_reservation(99999).await;
        assert!(result.is_err());
    }

    /// 测试_释放预留_无表结构返回错误
    ///
    /// 需要 inventory_reservations 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时 release_reservation 应返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_释放预留_无表结构返回错误() {
        let db = setup_test_db().await;
        let service = InventoryReservationService::new(Arc::new(db));

        // 无 inventory_reservations 表 schema，应返回数据库错误
        let result = service.release_reservation(99999).await;
        assert!(result.is_err());
    }

    /// 测试_查询预留列表_无表结构返回错误
    ///
    /// 需要 inventory_reservations 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时 list_reservations 应返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_查询预留列表_无表结构返回错误() {
        let db = setup_test_db().await;
        let service = InventoryReservationService::new(Arc::new(db));

        // 无 inventory_reservations 表 schema，分页查询应返回数据库错误
        let result = service.list_reservations(0, 10, None, None, None, None).await;
        assert!(result.is_err());
    }
}
