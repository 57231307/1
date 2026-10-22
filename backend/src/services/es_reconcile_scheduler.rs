//! ES 搜索索引对账补偿调度器
//!
//! 客户/产品/销售订单写入链路的 ES 同步失败路径均以"PG 已提交，最终一致性靠补偿任务修复"
//! 的文案兜底（客户更新/删除、产品增删改、销售订单增改删的同步 warn）。本模块即该承诺的
//! 权威补偿任务：以 PG 为唯一事实来源，周期性重建 customers / products / sales_orders 三个
//! 已同步索引，使"同步失败导致的 ES 缺档/脏档/孤儿档"在下一轮对账内收敛回 PG 口径。
//!
//! 对账策略（按索引形态分两档）：
//! - customers / products：PG 全量分页重推（ES 索引写入为幂等 upsert，可同时修复缺档与内容漂移），
//!   随后以 PG 主键集合反查 ES 现存文档、清除 PG 已不存在的孤儿文档；
//!   products 域仅重推未删除行（软删产品的 ES 文档由孤儿清除阶段负责移除）。
//! - sales_orders：文档内嵌明细行（重建每单需查明细与关联名），先按订单号对齐 PG 行与 ES 文档，
//!   比对 status / total_amount 两列识别"缺档或内容漂移"，仅对异常单走整单明细重建重推
//!   （复用写入链路的同源构建方法，保证文档口径与实时同步一致）；孤儿文档同轮删除。
//!   单轮重建封顶数 = 配置 batch_size，超出部分显式记入顺延日志。
//!
//! 已知边界（显式声明，不静默）：
//! - real ES 模式下检索分页受 ES 默认 max_result_window（10000）约束，超窗口文档本轮不被
//!   孤儿/漂移检查覆盖，会以 warn 明示（sales_orders 域同时因"ES 全量不可确认"跳过缺档判定，
//!   避免把未见到的文档误判为缺失而全量重建）；
//! - sales_orders 仅比对 status / total_amount：两列未变、仅明细字段漂移的文档无法在此档识别，
//!   该残留属显式口径而非漏报，全量重建属运维手动对账范畴。
//!
//! 生命周期：与其余启动期定时任务同框架（MAIN_BACKGROUND_TASKS 注册 + 全局取消 Token 优雅退出）。
//! `ELASTICSEARCH_URL` 未配置时搜索客户端为进程内 mock 存储（无跨进程最终一致性需求），
//! 启动链不挂载本任务；开关/间隔/批量上限来自配置层 `background_jobs.es_reconcile` 段。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::settings::EsReconcileJob;
use crate::models::{customer, product, sales_order};
use crate::search::{SearchClient, SearchQuery, SearchSyncer, indices};
use crate::services::customer_service::CustomerService;
use crate::services::product_service::ProductService;
use crate::services::so::order::SalesService;
use crate::services::so::order_crud;

/// ES 检索分页窗口上限：ES 默认 index.max_result_window=10000，from+size 超过即 400 报错。
/// 属 ES 协议侧约束（非业务可调参数），故为具名常量而不进配置段。
const ES_SEARCH_WINDOW_MAX: i64 = 10_000;

/// PG 侧销售订单快照行（对账判据：文档主键 order_no 对应的权威 status 与金额文档值）
struct OrderSnapshot {
    id: i32,
    status: String,
    total_f64: f64,
}

/// ES 对账补偿调度器
pub struct EsReconcileScheduler {
    db: Arc<DatabaseConnection>,
    client: Arc<dyn SearchClient>,
    job: EsReconcileJob,
}

impl EsReconcileScheduler {
    pub fn new(
        db: Arc<DatabaseConnection>,
        client: Arc<dyn SearchClient>,
        job: EsReconcileJob,
    ) -> Self {
        Self { db, client, job }
    }

    /// 执行一轮对账：三域各自独立执行，单域失败记入失败清单并继续下一域，轮末统一上抛供调度循环记 warn
    pub async fn scan_once(&self) -> Result<(), String> {
        let mut failures: Vec<String> = Vec::new();
        if let Err(e) = self.reconcile_customers().await {
            failures.push(format!("customers: {e}"));
        }
        if let Err(e) = self.reconcile_products().await {
            failures.push(format!("products: {e}"));
        }
        if let Err(e) = self.reconcile_sales_orders().await {
            failures.push(format!("sales_orders: {e}"));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("ES 对账本轮存在失败域: {}", failures.join("; ")))
        }
    }

    /// customers 域：PG 全量分页重推 + 以 PG 主键集合清除 ES 孤儿文档
    async fn reconcile_customers(&self) -> Result<(), String> {
        let batch = self.job.batch_size.max(1);
        let mut pg_ids: HashSet<String> = HashSet::new();
        let mut repushed: u64 = 0;
        let mut offset: u64 = 0;
        loop {
            let rows = customer::Entity::find()
                .order_by_asc(customer::Column::Id)
                .offset(offset)
                .limit(batch)
                .all(&*self.db)
                .await
                .map_err(|e| format!("分页查询 PG 客户失败: {e}"))?;
            if rows.is_empty() {
                break;
            }
            let mut docs: Vec<(String, serde_json::Value)> = Vec::with_capacity(rows.len());
            for row in &rows {
                pg_ids.insert(row.id.to_string());
                // 与写入链路同一构建方法（唯一派生源），对账口径 = 实时同步口径
                let doc = CustomerService::build_customer_doc(row);
                match serde_json::to_value(&doc) {
                    Ok(value) => docs.push((doc.id.to_string(), value)),
                    Err(e) => warn!(
                        customer_id = row.id,
                        error = %e,
                        "ES 对账：客户文档序列化失败，跳过该行（下一轮对账再试）"
                    ),
                }
            }
            repushed += self.bulk_upsert(indices::CUSTOMERS, &docs).await;
            offset += batch;
            if (rows.len() as u64) < batch {
                break;
            }
        }
        let orphans_deleted = self.sweep_orphans(indices::CUSTOMERS, &pg_ids).await?;
        info!(
            index = indices::CUSTOMERS,
            pg_rows = pg_ids.len(),
            repushed,
            orphans_deleted,
            "ES 对账 customers 域本轮完成"
        );
        Ok(())
    }

    /// products 域：PG 未删除行全量分页重推 + 清除 ES 孤儿文档（软删产品不重推，其文档由孤儿清除移除）
    async fn reconcile_products(&self) -> Result<(), String> {
        let batch = self.job.batch_size.max(1);
        let product_service = ProductService::new(self.db.clone(), self.client.clone());
        let mut pg_ids: HashSet<String> = HashSet::new();
        let mut repushed: u64 = 0;
        let mut offset: u64 = 0;
        loop {
            let rows = product::Entity::find()
                .filter(product::Column::IsDeleted.eq(false))
                .order_by_asc(product::Column::Id)
                .offset(offset)
                .limit(batch)
                .all(&*self.db)
                .await
                .map_err(|e| format!("分页查询 PG 产品失败: {e}"))?;
            if rows.is_empty() {
                break;
            }
            let mut docs: Vec<(String, serde_json::Value)> = Vec::with_capacity(rows.len());
            for row in &rows {
                pg_ids.insert(row.id.to_string());
                let doc = product_service.build_product_doc(row);
                match serde_json::to_value(&doc) {
                    Ok(value) => docs.push((doc.id.to_string(), value)),
                    Err(e) => warn!(
                        product_id = row.id,
                        error = %e,
                        "ES 对账：产品文档序列化失败，跳过该行（下一轮对账再试）"
                    ),
                }
            }
            repushed += self.bulk_upsert(indices::PRODUCTS, &docs).await;
            offset += batch;
            if (rows.len() as u64) < batch {
                break;
            }
        }
        let orphans_deleted = self.sweep_orphans(indices::PRODUCTS, &pg_ids).await?;
        info!(
            index = indices::PRODUCTS,
            pg_rows = pg_ids.len(),
            repushed,
            orphans_deleted,
            "ES 对账 products 域本轮完成"
        );
        Ok(())
    }

    /// sales_orders 域：PG 快照（order_no → status/total）与 ES 文档对齐，
    /// 仅对缺档/漂移单走整单明细重建重推（封顶 batch_size），孤儿文档同轮删除
    async fn reconcile_sales_orders(&self) -> Result<(), String> {
        let batch = self.job.batch_size.max(1);
        let mut pg_snapshot: HashMap<String, OrderSnapshot> = HashMap::new();
        let mut offset: u64 = 0;
        loop {
            let rows = sales_order::Entity::find()
                .order_by_asc(sales_order::Column::Id)
                .offset(offset)
                .limit(batch)
                .all(&*self.db)
                .await
                .map_err(|e| format!("分页查询 PG 销售订单失败: {e}"))?;
            let got = rows.len();
            for row in rows {
                pg_snapshot.insert(
                    row.order_no.clone(),
                    OrderSnapshot {
                        id: row.id,
                        status: row.status.clone(),
                        total_f64: order_crud::decimal_to_f64(&row.total_amount),
                    },
                );
            }
            offset += batch;
            if got < batch as usize {
                break;
            }
        }

        let size = batch as i64;
        let mut from: i64 = 0;
        let mut es_seen_ids: HashSet<String> = HashSet::new();
        let mut need_rebuild: Vec<i32> = Vec::new();
        let mut orphans_deleted: u64 = 0;
        let mut sweep_complete = true;
        let mut window_warned = false;
        loop {
            let query = SearchQuery {
                q: None,
                filters: HashMap::new(),
                from,
                size,
                highlight: false,
            };
            let result = match self.client.search(indices::SALES_ORDERS, &query).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(
                        from,
                        error = %e,
                        "ES 对账：sales_orders 检索失败（多为超出 ES 分页窗口），本轮仅覆盖前 {} 档；缺档判定跳过，防误全量重建",
                        from
                    );
                    sweep_complete = false;
                    break;
                }
            };
            if !window_warned && result.total > ES_SEARCH_WINDOW_MAX {
                warn!(
                    es_total = result.total,
                    window = ES_SEARCH_WINDOW_MAX,
                    "ES 对账：sales_orders 文档总量超出检索窗口，本轮漂移/孤儿检查仅覆盖窗口内文档"
                );
                window_warned = true;
            }
            let hits_len = result.hits.len();
            for hit in result.hits {
                es_seen_ids.insert(hit.id.clone());
                match pg_snapshot.get(&hit.id) {
                    Some(snap) => {
                        let es_status = hit.source.get("status").and_then(|v| v.as_str());
                        let es_total = hit.source.get("total_amount").and_then(|v| v.as_f64());
                        // 判据：状态逐字符不同，或金额文档值偏离 PG 权威值（epsilon 比较，避免浮点相等告警）。
                        // 命中判据即走整单明细重建重推（重建幂等，宁可误重建也不漏修脏档）。
                        let drift = match (es_status, es_total) {
                            (Some(st), Some(t)) => {
                                st != snap.status.as_str()
                                    || (t - snap.total_f64).abs() > f64::EPSILON
                            }
                            _ => true,
                        };
                        if drift {
                            need_rebuild.push(snap.id);
                        }
                    }
                    None => match self.client.delete_doc(indices::SALES_ORDERS, &hit.id).await {
                        Ok(()) => orphans_deleted += 1,
                        Err(e) => warn!(
                            doc_id = %hit.id,
                            error = %e,
                            "ES 对账：sales_orders 孤儿文档删除失败（下一轮对账再试）"
                        ),
                    },
                }
            }
            from += size;
            if (hits_len as i64) < size || from >= result.total {
                break;
            }
        }

        // 缺档判定仅在 ES 全量遍历成功时可信（窗口截断时未见到 ≠ 不存在）
        if sweep_complete {
            for (order_no, snap) in &pg_snapshot {
                if !es_seen_ids.contains(order_no) {
                    need_rebuild.push(snap.id);
                }
            }
        } else {
            warn!(
                pg_orders = pg_snapshot.len(),
                "ES 对账：sales_orders 本轮 ES 全量未确认，缺档判定跳过（下一轮对账覆盖）"
            );
        }

        need_rebuild.sort_unstable();
        need_rebuild.dedup();
        let pending = need_rebuild.len();
        let take: Vec<i32> = need_rebuild.into_iter().take(batch as usize).collect();
        if pending > take.len() {
            warn!(
                pending,
                rebuild_this_round = take.len(),
                deferred = pending - take.len(),
                "ES 对账：sales_orders 待重建文档超出单轮封顶（batch_size），剩余顺延下一轮"
            );
        }

        let syncer = SearchSyncer::new(self.client.clone());
        let sales_service = SalesService::new(self.db.clone(), self.client.clone());
        let mut rebuilt: u64 = 0;
        let mut rebuild_failed: u64 = 0;
        for order_id in take {
            match sales_service.get_order_detail(order_id, None).await {
                Ok(detail) => {
                    let doc = SalesService::build_sales_order_doc(&detail);
                    match syncer.sync_sales_order(&doc).await {
                        Ok(()) => rebuilt += 1,
                        Err(e) => {
                            rebuild_failed += 1;
                            warn!(
                                order_id,
                                order_no = %detail.order_no,
                                error = %e,
                                "ES 对账：sales_orders 文档重建重推失败（下一轮对账再试）"
                            );
                        }
                    }
                }
                Err(e) => {
                    rebuild_failed += 1;
                    warn!(
                        order_id,
                        error = %e,
                        "ES 对账：sales_orders 读取整单明细失败，跳过该单（下一轮对账再试）"
                    );
                }
            }
        }
        info!(
            index = indices::SALES_ORDERS,
            pg_orders = pg_snapshot.len(),
            drift_or_missing = pending,
            rebuilt,
            rebuild_failed,
            orphans_deleted,
            "ES 对账 sales_orders 域本轮完成"
        );
        Ok(())
    }

    /// 批量幂等重推：返回实际被接收的文档数；批级失败与部分未接收均显式记 warn（不静默）
    async fn bulk_upsert(&self, index: &str, docs: &[(String, serde_json::Value)]) -> u64 {
        if docs.is_empty() {
            return 0;
        }
        match self.client.bulk_index(index, docs).await {
            Ok(n) => {
                if n < docs.len() {
                    warn!(
                        index,
                        expected = docs.len(),
                        indexed = n,
                        "ES 对账：批量索引仅部分文档被接收（未接收文档等待下一轮对账再推）"
                    );
                }
                n as u64
            }
            Err(e) => {
                warn!(
                    index,
                    batch_docs = docs.len(),
                    error = %e,
                    "ES 对账：批量索引整批失败（本批等待下一轮对账再推）"
                );
                0
            }
        }
    }

    /// 反查指定索引：删除 PG 权威集合中不存在的孤儿文档。
    /// 检索分页失败（常见为超出 ES 窗口）时记 warn 并止步于已覆盖范围，返回已删除计数。
    async fn sweep_orphans(&self, index: &str, pg_ids: &HashSet<String>) -> Result<u64, String> {
        let size = self.job.batch_size.max(1) as i64;
        let mut deleted: u64 = 0;
        let mut from: i64 = 0;
        let mut window_warned = false;
        loop {
            let query = SearchQuery {
                q: None,
                filters: HashMap::new(),
                from,
                size,
                highlight: false,
            };
            let result = match self.client.search(index, &query).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(
                        index,
                        from,
                        error = %e,
                        "ES 对账：孤儿检查检索失败（多为超出 ES 分页窗口），本轮孤儿检查止于前 {} 档",
                        from
                    );
                    break;
                }
            };
            if !window_warned && result.total > ES_SEARCH_WINDOW_MAX {
                warn!(
                    index,
                    es_total = result.total,
                    window = ES_SEARCH_WINDOW_MAX,
                    "ES 对账：文档总量超出检索窗口，本轮孤儿检查仅覆盖窗口内文档"
                );
                window_warned = true;
            }
            let hits_len = result.hits.len();
            for hit in result.hits {
                if !pg_ids.contains(&hit.id) {
                    match self.client.delete_doc(index, &hit.id).await {
                        Ok(()) => deleted += 1,
                        Err(e) => warn!(
                            index,
                            doc_id = %hit.id,
                            error = %e,
                            "ES 对账：孤儿文档删除失败（下一轮对账再试）"
                        ),
                    }
                }
            }
            from += size;
            if (hits_len as i64) < size || from >= result.total {
                break;
            }
        }
        Ok(deleted)
    }

    /// 启动后台调度循环（受全局取消 Token 控制），返回 JoinHandle 供启动链注册与停机 abort。
    pub fn start_background_task(
        self: Arc<Self>,
        cancel: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(self.job.initial_delay_secs)).await;
            let interval = std::time::Duration::from_secs(self.job.interval_secs);
            info!(
                interval_secs = self.job.interval_secs,
                batch_size = self.job.batch_size,
                "ES 对账补偿调度运行中（每 {} 秒以 PG 为权威源重建 customers/products/sales_orders 索引）",
                self.job.interval_secs
            );
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        match self.scan_once().await {
                            Ok(()) => {}
                            Err(e) => warn!(error = %e, "ES 对账补偿调度：本轮存在失败域，下一轮继续"),
                        }
                    }
                    _ = cancel.cancelled() => {
                        info!("ES 对账补偿调度收到取消信号，优雅退出");
                        break;
                    }
                }
            }
        })
    }
}
