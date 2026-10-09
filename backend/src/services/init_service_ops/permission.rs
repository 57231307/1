//! 角色权限矩阵与互斥规则子模块（init_service_ops/permission）
//!
//! 从原 `init_service.rs` 迁移 2 个方法：
//! - create_default_role_permissions：为全部角色创建 role_permission 权限矩阵（覆盖 60+ 资源 × 11 操作码）
//! - create_default_role_conflicts：初始化默认角色互斥规则（SoD 职责分离）
//!
//! 授权落库口径（E6 根修，本模块唯一判据）：矩阵的幂等按**角色 × (resource,action)
//! 逐条对账**，不再用"role_permissions 表整体有行就跳过"；三类缺口一律 fail-visible 点名报错，
//! 不许静默 skip 后返回成功——① 资源码不在 `PERMISSION_RESOURCES` 注册表（授予也是死码）、
//! ② 角色码在 roles 表解析不到（上游建角色链断）、③ 写后"应写 vs 实写"对账不足。
//! 授权语义（哪个角色该有哪些资源码）是本模块定义表的唯一职责：改授予面 = 改本表，
//! 且新角色码必须同步在 `role.rs::create_default_roles` 在册、新资源码必须已在
//! `PERMISSION_RESOURCES` 登记，闸门①②会对违约直接判红。
//!
//! E6 的因果分工（避免下个改动把根因认错）：CI 里业务授权没落库的**直接**原因是
//! `init_service_ops/role.rs::create_default_roles` 旧写法"admin 已存在即整体早退"，
//! 矩阵 37 个角色码里只有 2 个能在 roles 表解析到；本模块的 `count>0` 整体跳过守卫是
//! **第二道**会吞授权的雷（存量库或已被迁移/e2e 授过部分码时重放 init 必然吞掉首建），
//! 两道都已闭合，行为级证据见 `tests/services_init_role_permission_matrix_test.rs`
//! （①②③ 各钉一条：真库回读授权行确实存在、重放逐条等价、缺角色时点名判红且零写入）。
use std::collections::{HashMap, HashSet};

use crate::models::{role, role_conflict, role_permission};
use crate::services::init_service::{InitError, InitService, PERMISSION_RESOURCES};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set};
use tracing::warn;

// ===== 类型别名（避免 clippy `type_complexity` 警告：嵌套引用切片类型过深）=====
//
// 前三个别名 `pub` 的必要性：`find_unregistered_matrix_resources` 是注册表 fail-visible
// 闸门，除生产链内部调用外还要能被集成测试用"未登记资源码"直接驱动负向断言
// （tests/services_init_role_permission_matrix_test.rs）。闸门只有一道实现，
// 测试喂的是同一道闸门的入参形态，不存在第二套判定。

/// 资源-操作对（如 ("users", "read")）
pub type PermPair = (&'static str, &'static str);

/// 角色权限定义组：角色代码 + 该角色的资源操作列表
pub type RoleResourceGroup = (&'static str, &'static [PermPair]);

/// 单个域的角色权限定义切片（多个角色权限组）
pub type RoleResourceSlice = &'static [RoleResourceGroup];

/// 全部域的角色权限定义分组列表（仅本模块内部聚合用，不进公开签名）
type RoleResourceGroups = Vec<RoleResourceSlice>;

/// 应用外壳权限码：每个角色都必须具备的两项。
///
/// - `dashboard:read`：登录后落地页就是 `/dashboard`（路由 `/` 重定向到它），
///   卡片数据由 handler 按角色数据范围（new_with_data_scope）过滤，
///   财务卡片仅财务角色可见，因此读权限不会放大可见数据。
/// - `notifications:read`：主框架铃铛对全部认证用户拉取自身未读数，
///   handler 只按 `auth.user_id` 取该用户自己的通知行。
///
/// 缺码的角色登录后会被路由守卫送到 /403，或每次进主页都在控制台抛 403。
const SHELL_PERMISSIONS: &[PermPair] = &[("dashboard", "read"), ("notifications", "read")];

/// 库里"已经算授权到位"的 role_permission 索引（按角色归组的 (resource,action) 集合）。
///
/// 判据必须是**角色级 + allowed=true**，不是"同键有行就算"：
/// - 记录级行（`resource_id = Some(..)`）按 `matches_permission`
///   （`middleware/permission.rs:619-637`：`(Some(_), None) => false`）不能放行整集合请求。
///   若拿它顶替角色级授权，矩阵会跳过写入、生产工单列表 403 依旧，而 init 报"成功"——
///   这正是"半截授权被当成已授权"的形态之一，故记录级行两侧都不计入。
/// - `allowed = false` 是显式拒绝型权限（`role_permission_service.rs:366` 口径：
///   "allowed=false 的拒绝型权限不构成冲突"），矩阵不得覆盖它（覆盖＝放大授权面，
///   属授权语义变更，不在本修范围），但必须点名留痕，不得静默当成已授权。
struct ExistingRoleGrants {
    /// 角色级、allowed=true：矩阵幂等跳过的依据
    granted: HashMap<i32, HashSet<(String, String)>>,
    /// 角色级、allowed=false：矩阵不覆盖，逐条点名留痕
    denied: HashMap<i32, HashSet<(String, String)>>,
}

/// (角色码, 资源码, 操作码) 三元组，仅用于缺口点名
type PermTriple = (&'static str, &'static str, &'static str);

/// 一轮授权建立的差集计划（应写集合 + 待点名缺口）。
struct PermissionWritePlan {
    /// 待写模型（库里缺失的角色级授权）
    to_insert: Vec<role_permission::ActiveModel>,
    /// 本次授权涉及的角色 ID（写后对账按这批角色计数）
    pending_role_ids: Vec<i32>,
    /// 应写条数（= to_insert 长度，另存一份供对账与日志）
    expected_total: usize,
    /// 在 roles 表解析不到的角色码（闸门②点名对象）
    missing_role_codes: Vec<&'static str>,
    /// 矩阵想授、库里却是同键显式拒绝行的组合
    explicit_denied_kept: Vec<PermTriple>,
}

/// 未登记资源码的点名结果：(资源码, 引用它的角色码列表)
pub type UnregisteredResource = (String, Vec<String>);

/// 找出矩阵定义里未登记在 `PERMISSION_RESOURCES` 中的资源码（闸门①的判据）。
///
/// 纯函数、不碰库，返回 `Vec<(资源码, 引用它的角色码列表)>`，空集即全部合法。
/// 生产闸门 `InitService::assert_matrix_resources_registered` 与集成测试共用这同一份
/// 判据（不造第二套判定）：测试可直接喂未登记资源码驱动负向断言，只判返回集合、不判文案。
/// 写成模块级自由函数而非 `InitService` 的关联函数：它不属于任何实例，也不建实例。
pub fn find_unregistered_matrix_resources(
    groups: &[RoleResourceSlice],
) -> Vec<UnregisteredResource> {
    let registry: HashSet<&str> = PERMISSION_RESOURCES.iter().copied().collect();
    let mut unknown: Vec<UnregisteredResource> = Vec::new();
    for definition in groups {
        for (role_code, resources) in *definition {
            for (resource, _action) in *resources {
                if registry.contains(resource) {
                    continue;
                }
                match unknown.iter_mut().find(|(res, _)| res == resource) {
                    Some((_, roles)) => {
                        if !roles.contains(&role_code.to_string()) {
                            roles.push(role_code.to_string());
                        }
                    }
                    None => unknown.push((resource.to_string(), vec![role_code.to_string()])),
                }
            }
        }
    }
    unknown
}

impl InitService {
    /// 创建全部角色的 role_permission 权限矩阵（V15 P0-S03/S04/S20，覆盖 60+ 资源 × 11 操作码）。
    ///
    /// 幂等与可观测性口径（根修，见本模块顶部说明）
    /// - 幂等**按角色逐条 (resource,action) 对账**：库里已有的角色级授权跳过（可重入、不重复插），
    ///   库里缺失的组合补齐。旧的全表 `count>0 即整体跳过` 会把"迁移/e2e 半截补建的少量授权行"
    ///   误当成全矩阵已落地，从而吞掉 production-orders 等业务授权的首建（本条是 E6 的第二道雷，
    ///   直接原因见模块头"E6 的因果分工"），予以废除。
    /// - fail-visible 三道闸门，任一不过一律点名报错，禁止静默 skip 后返回成功：
    ///   ① 矩阵引用的资源码必须已登记在 `PERMISSION_RESOURCES`（未登记=写进库也永不生效的死码授权）；
    ///   ② 角色码必须在 roles 表解析得到（解析不到=上游建角色链断，该角色授权一条都写不进）；
    ///   ③ 写后按"应写 vs 实写"对账，实写 < 应写即落库缺口。
    /// - 同键显式拒绝行不被覆盖，但逐条点名留痕。
    ///
    /// 可见性 `pub`：生产路径仍只由 `initialize` 链内部调用；放开是为了让集成测试能在
    /// 真库上行为级驱动"授权到底有没有落库"（tests/services_init_role_permission_matrix_test.rs），
    /// 而不是让测试另造一套等价实现来"自证"。
    pub async fn create_default_role_permissions(&self) -> Result<(), InitError> {
        let db = self.db.as_ref();
        let now = chrono::Utc::now();

        // 闸门①：纯静态校验，放在任何写之前——资源码没登记就别写，写了也是死码。
        Self::assert_matrix_resources_registered()?;

        // 一次性拉取现有授权并按 role_id 归组，避免逐角色 N+1。
        let existing = Self::load_existing_permissions(db).await?;

        // 逐角色解析、diff 出库里缺失的授权；缺口只收集不静默丢弃。
        let PermissionWritePlan {
            to_insert,
            pending_role_ids,
            expected_total,
            missing_role_codes,
            explicit_denied_kept,
        } = Self::collect_missing_permission_writes(db, &existing, now).await?;

        // 闸门②：任一角色码在 roles 表解析不到 = 授权建立上游断链，必须红。
        if !missing_role_codes.is_empty() {
            return Err(InitError::DatabaseError(format!(
                "角色权限矩阵建立不完整：{} 个角色码在 roles 表解析不到、其授权一条都未写入：{:?}",
                missing_role_codes.len(),
                missing_role_codes
            )));
        }

        // 显式拒绝保持不覆盖（不改授权语义），但必须点名——否则就是"半截授权被当成已授权"。
        if !explicit_denied_kept.is_empty() {
            let samples: Vec<String> = explicit_denied_kept
                .iter()
                .take(10)
                .map(|(r, res, act)| format!("{}:{}.{}", r, res, act))
                .collect();
            warn!(
                "角色权限矩阵存在同键显式拒绝行（allowed=false），按最小惊讶原则不覆盖、本批未补写授权：{} 条，样例 {:?}",
                explicit_denied_kept.len(),
                samples
            );
        }

        let mut written = 0usize;
        if !to_insert.is_empty() {
            written = Self::persist_and_reconcile(db, to_insert, pending_role_ids, expected_total)
                .await?;
        }

        tracing::info!(
            "角色权限矩阵建立完成：应写 {} 条、实写 {} 条（库里已授权的角色级组合按条幂等跳过，同键显式拒绝 {} 条保持不覆盖）",
            expected_total,
            written,
            explicit_denied_kept.len()
        );
        Ok(())
    }

    /// 闸门①：矩阵（含应用外壳码）引用的资源码必须已在 `PERMISSION_RESOURCES` 登记。
    ///
    /// 为什么算缺口而不是"多余授权"：运行时权限键由 URL 段推导并与注册表同源
    /// （`middleware/permission.rs::validate_route_whitelist` + `utils/path_utils.rs`），
    /// 没登记的资源段一律被判"未知的资源路径"403 ⇒ 该授权行写进 `role_permissions`
    /// 也永不命中，属于"看起来授权过、其实那条授权是死码"的静默缺陷，与授权没落库同罪。
    fn assert_matrix_resources_registered() -> Result<(), InitError> {
        let mut groups = Self::all_role_permission_definition_groups();
        // 应用外壳码同样是矩阵写入的一部分，一并纳入闸门校验（标一个可读的来源标签，
        // 便于缺口点名时直接看出是外壳码没登记，而不是某个业务角色的码表写错）
        groups.push(&[("ALL·应用外壳码", SHELL_PERMISSIONS)]);
        let unknown = find_unregistered_matrix_resources(&groups);
        if unknown.is_empty() {
            return Ok(());
        }
        Err(InitError::DatabaseError(format!(
            "角色权限矩阵存在未登记的资源码 {} 个（授予后运行期永不命中，等于没授权）：{}",
            unknown.len(),
            unknown
                .iter()
                .map(|(res, roles)| format!("{}(被引用角色 {:?})", res, roles))
                .collect::<Vec<String>>()
                .join("、")
        )))
    }

    /// 拉取现有全部 role_permission 行，按 role_id 归组为 (resource_type, action) 集合。
    /// 供逐条幂等 diff 使用，单次查询避免 N+1。判据见 [`ExistingRoleGrants`]。
    async fn load_existing_permissions(
        db: &DatabaseConnection,
    ) -> Result<ExistingRoleGrants, InitError> {
        let rows = role_permission::Entity::find().all(db).await.map_err(|e| {
            InitError::DatabaseError(format!("查询现有角色权限失败，无法对账: {}", e))
        })?;
        let mut granted: HashMap<i32, HashSet<(String, String)>> = HashMap::new();
        let mut denied: HashMap<i32, HashSet<(String, String)>> = HashMap::new();
        for row in rows {
            // 记录级授权与矩阵的角色级授权不是同一维度：既不顶替、也不算拒绝
            if row.resource_id.is_some() {
                continue;
            }
            let bucket = if row.allowed {
                &mut granted
            } else {
                &mut denied
            };
            bucket
                .entry(row.role_id)
                .or_default()
                .insert((row.resource_type, row.action));
        }
        Ok(ExistingRoleGrants { granted, denied })
    }

    /// 汇总全部域的角色权限定义分组（管理/高管/销售/采购/库存/生产/质量/财务/CRM物流HR/其他）。
    ///
    /// 可见性 `pub` 的必要性：approve/reject 成对性锁（tests/contract_wave11_approve_reject_pairing_test.rs）
    /// 要判的就是**这一份**定义表；放开入口让测试直接驱动生产同一份数据，
    /// 不在 tests/ 侧另抄一份矩阵副本造成第二套判定（同 `find_unregistered_matrix_resources` 的口径）。
    pub fn all_role_permission_definition_groups() -> RoleResourceGroups {
        vec![
            Self::management_role_resources(),
            Self::executive_role_resources(),
            Self::sales_role_resources(),
            Self::purchase_role_resources(),
            Self::inventory_role_resources(),
            Self::production_core_role_resources(),
            Self::production_extended_role_resources(),
            Self::quality_role_resources(),
            Self::finance_role_resources(),
            Self::crm_logistics_hr_role_resources(),
            Self::misc_role_resources(),
        ]
    }

    /// 遍历全部域的角色权限定义，逐角色解析并 diff 出"库里缺失"的授权待写集合。
    ///
    /// 角色解析不到者不静默丢弃，而是记入 `missing_role_codes` 交调用方判红；库里已存在的
    /// **角色级 allowed=true** (resource,action) 逐条跳过，已完整授权的角色自然 0 补写
    /// （可重入），半截授权的角色只补齐缺失项（不吞首建）；同键显式拒绝行记入
    /// `explicit_denied_kept` 点名留痕、不覆盖。
    async fn collect_missing_permission_writes(
        db: &DatabaseConnection,
        existing: &ExistingRoleGrants,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<PermissionWritePlan, InitError> {
        let mut to_insert: Vec<role_permission::ActiveModel> = Vec::new();
        let mut pending_role_ids: Vec<i32> = Vec::new();
        let mut missing_role_codes: Vec<&'static str> = Vec::new();
        let mut explicit_denied_kept: Vec<PermTriple> = Vec::new();
        let mut expected_total: usize = 0;

        for definitions in Self::all_role_permission_definition_groups() {
            for (role_code, resources) in definitions {
                // 应用外壳码：每个角色都必须具备 dashboard/notifications，缺则登录后停在 /403。
                let mut effective: Vec<PermPair> = resources.to_vec();
                for shell in SHELL_PERMISSIONS {
                    if !effective.contains(shell) {
                        effective.push(*shell);
                    }
                }

                let role_model = role::Entity::find()
                    .filter(role::Column::Code.eq(*role_code))
                    .one(db)
                    .await
                    .map_err(|e| {
                        InitError::DatabaseError(format!("查询 {} 角色失败: {}", role_code, e))
                    })?;

                // 角色解析不到：旧写法 `if let Some(r) {..}` 无 else 分支，会静默跳过、
                // 一条不写也不报错（本仓硬教训：init 找不到 code 只 warn+skip =
                // "看起来跑过了其实没写"）。这里点名收集，交由调用方 fail-visible 判红。
                let Some(role) = role_model else {
                    missing_role_codes.push(*role_code);
                    continue;
                };

                // 逐条 diff：仅保留库里缺失"角色级 allowed=true"的 (resource,action) 组合。
                let granted = existing.granted.get(&role.id);
                let denied = existing.denied.get(&role.id);
                let mut missing_perms: Vec<PermPair> = Vec::new();
                for (res, act) in effective {
                    let key = (res.to_string(), act.to_string());
                    if granted.is_some_and(|set| set.contains(&key)) {
                        continue;
                    }
                    if denied.is_some_and(|set| set.contains(&key)) {
                        // 库里是同键显式拒绝：覆盖它会放大授权面（语义变更，本修不做），
                        // 但绝不静默——点名交给调用方留痕。
                        explicit_denied_kept.push((*role_code, res, act));
                        continue;
                    }
                    missing_perms.push((res, act));
                }

                if missing_perms.is_empty() {
                    // 该角色已按矩阵完整授权（可重入分支），跳过。
                    continue;
                }

                expected_total += missing_perms.len();
                pending_role_ids.push(role.id);
                to_insert.extend(Self::make_permission_models(role.id, &missing_perms, now));
            }
        }

        Ok(PermissionWritePlan {
            to_insert,
            pending_role_ids,
            expected_total,
            missing_role_codes,
            explicit_denied_kept,
        })
    }

    /// 落库并做"应写 vs 实写"对账：写失败或实写条数不足应写条数，一律点名判红。
    ///
    /// 返回**实写条数**给调用方留痕，避免日志把"应写"当"已写"播报。
    async fn persist_and_reconcile(
        db: &DatabaseConnection,
        to_insert: Vec<role_permission::ActiveModel>,
        pending_role_ids: Vec<i32>,
        expected_total: usize,
    ) -> Result<usize, InitError> {
        // 写前基线：这些待补角色当前已有行数（写后精确算实写条数用）。
        let before = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.is_in(pending_role_ids.clone()))
            .count(db)
            .await
            .map_err(|e| InitError::DatabaseError(format!("写前统计角色权限基线失败: {}", e)))?;

        // 落库失败直接上抛（写不进 = 授权没落库 = 必须红），不再只 warn 后假绿。
        role_permission::Entity::insert_many(to_insert)
            .exec(db)
            .await
            .map_err(|e| {
                InitError::DatabaseError(format!(
                    "批量写入角色权限失败（本轮应写 {} 条）: {}",
                    expected_total, e
                ))
            })?;

        // 对账：写后总行数 - 写前基线 = 实写；不足应写即有行未真正落库（约束/触发器/静默丢行），
        // 缺口显式化并判红。
        let after = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.is_in(pending_role_ids))
            .count(db)
            .await
            .map_err(|e| InitError::DatabaseError(format!("写后角色权限对账计数失败: {}", e)))?;
        let written = after.saturating_sub(before);
        if written < expected_total as u64 {
            return Err(InitError::DatabaseError(format!(
                "角色权限落库对账缺口：应写 {} 条，实写 {} 条（差额 {} 条未落库）",
                expected_total,
                written,
                expected_total as u64 - written
            )));
        }
        Ok(written as usize)
    }

    /// 为指定角色按 resource×action 列表生成权限 ActiveModel（全部 allowed=true）。
    fn make_permission_models(
        role_id: i32,
        resources: &[(&str, &str)],
        now: chrono::DateTime<chrono::Utc>,
    ) -> Vec<role_permission::ActiveModel> {
        resources
            .iter()
            .map(|(resource, action)| role_permission::ActiveModel {
                id: Default::default(),
                role_id: Set(role_id),
                resource_type: Set(resource.to_string()),
                resource_id: Set(None),
                action: Set(action.to_string()),
                allowed: Set(true),
                permission_code: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .collect()
    }

    // ===== 角色权限数据定义（按域分组，每组 ≤50 行）=====

    /// 管理域角色权限定义（manager 部门经理跨域读取+本域全部操作 / operator 全业务域只读）。
    fn management_role_resources() -> RoleResourceSlice {
        &[
            // manager：部门经理，跨业务域读取 + 本域全部操作
            (
                "manager",
                &[
                    ("users", "read"),
                    ("roles", "read"),
                    ("departments", "read"),
                    ("products", "*"),
                    ("orders", "*"),
                    ("customers", "*"),
                    ("suppliers", "*"),
                    ("inventory", "read"),
                    ("stock-alerts", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("quality-inspections", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("gl", "read"),
                    ("reports", "read"),
                    ("dashboard", "read"),
                    ("export-approvals", "create"),
                ],
            ),
            // operator：操作员，全业务域只读
            (
                "operator",
                &[
                    ("products", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("quality-inspections", "read"),
                    ("reports", "read"),
                ],
            ),
        ]
    }

    /// 高管域角色权限定义（gm 总经理 / deputy_gm 副总经理，全资源 read + AI 域只读）。
    fn executive_role_resources() -> RoleResourceSlice {
        // V15 P0-S26：AI 域只读权限（管理层可查看所有 AI 分析结果）
        &[
            (
                "gm",
                &[
                    ("users", "read"),
                    ("roles", "read"),
                    ("departments", "read"),
                    ("products", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("dye-recipes", "read"),
                    ("quality-inspections", "read"),
                    ("fabric-inspections", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("gl", "read"),
                    ("budgets", "read"),
                    ("cost-collections", "read"),
                    ("crm-leads", "read"),
                    ("crm-opportunities", "read"),
                    ("logistics", "read"),
                    ("employees", "read"),
                    ("wages", "read"),
                    ("reports", "read"),
                    ("bi-analysis", "read"),
                    ("dashboard", "read"),
                    ("ai-forecast", "read"),
                    ("ai-inventory-opt", "read"),
                    ("ai-anomaly", "read"),
                    ("ai-recommendation", "read"),
                    ("ai-recipe-opt", "read"),
                    ("ai-quality-pred", "read"),
                    ("ai-process-opt", "read"),
                    ("ai-summary", "read"),
                ],
            ),
            (
                "deputy_gm",
                &[
                    ("users", "read"),
                    ("roles", "read"),
                    ("departments", "read"),
                    ("products", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("dye-recipes", "read"),
                    ("quality-inspections", "read"),
                    ("fabric-inspections", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("gl", "read"),
                    ("budgets", "read"),
                    ("cost-collections", "read"),
                    ("crm-leads", "read"),
                    ("crm-opportunities", "read"),
                    ("logistics", "read"),
                    ("employees", "read"),
                    ("wages", "read"),
                    ("reports", "read"),
                    ("bi-analysis", "read"),
                    ("dashboard", "read"),
                    ("ai-forecast", "read"),
                    ("ai-inventory-opt", "read"),
                    ("ai-anomaly", "read"),
                    ("ai-recommendation", "read"),
                    ("ai-recipe-opt", "read"),
                    ("ai-quality-pred", "read"),
                    ("ai-process-opt", "read"),
                    ("ai-summary", "read"),
                ],
            ),
        ]
    }

    /// 销售域角色权限定义（V15 P1-14.3-D：sales_manager 仅审批不创建 / sales_rep 仅创建不审批）。
    fn sales_role_resources() -> RoleResourceSlice {
        &[
            // V15 P1-14.3-D：sales_manager 拆分 create 与 approve（SoD 职责分离）
            // 原为 ("orders", "*") 含 create+approve，现改为显式列出审批操作，移除 create
            (
                "sales_manager",
                &[
                    ("orders", "read"),
                    ("orders", "update"),
                    ("orders", "delete"),
                    ("orders", "approve"),
                    ("orders", "reject"),
                    ("fabric-orders", "read"),
                    ("fabric-orders", "approve"),
                    ("fabric-orders", "reject"),
                    // 拒绝键与审批键成对授予同一批角色：/reject 端点与 /approve 端点
                    // 一一对应，缺一侧即前端按钮可达而 RBAC 恒 403（或反向授权悬空）；
                    // 成对性由 tests/contract_wave11_approve_reject_pairing_test.rs 双向钉。
                    ("sales-contracts", "read"),
                    ("sales-contracts", "approve"),
                    ("sales-contracts", "reject"),
                    ("sales-prices", "read"),
                    ("sales-prices", "approve"),
                    ("sales-prices", "reject"),
                    ("sales-returns", "read"),
                    ("sales-returns", "approve"),
                    ("sales-returns", "reject"),
                    ("quotations", "read"),
                    ("quotations", "approve"),
                    ("quotations", "reject"),
                    ("custom-orders", "*"),
                    ("color-cards", "*"),
                    ("color-prices", "*"),
                    ("customers", "*"),
                    ("customer-credits", "read"),
                    ("ar", "read"),
                    ("reports", "read"),
                    ("sales-analysis", "read"),
                ],
            ),
            (
                "sales_rep",
                &[
                    ("orders", "read"),
                    ("orders", "create"),
                    ("orders", "update"),
                    ("fabric-orders", "read"),
                    ("fabric-orders", "create"),
                    ("quotations", "read"),
                    ("quotations", "create"),
                    ("customers", "read"),
                    ("customers", "create"),
                    // PII 按需揭示（POST /crm/customers/{id}/pii/reveal 的运行时键，
                    // 与 m0086 存量补授/e2e SEED 三通道同口径；每次揭示强制留痕）
                    ("customers", "reveal"),
                    ("sales-returns", "read"),
                    ("sales-returns", "create"),
                    ("color-cards", "read"),
                    ("sales-prices", "read"),
                    // 发货对话框第四维匹号候选来自 GET /inventory/pieces（只读，不授打印）
                    ("pieces", "read"),
                    // 出口商检：销售代表可读自己出口订单对应的商检单与结论（只读，
                    // 建单/登记结果属报关岗，不越界授予）。
                    ("export-inspections", "read"),
                ],
            ),
        ]
    }

    /// 采购域角色权限定义（V15 P1-14.3-D：purchase_manager 仅审批不创建 / purchase_clerk 仅创建不审批）。
    fn purchase_role_resources() -> RoleResourceSlice {
        &[
            // V15 P1-14.3-D：purchase_manager 拆分 create 与 approve（SoD 职责分离）
            // 原为 ("purchase-orders", "*") 含 create+approve，现改为显式列出审批操作，移除 create
            (
                "purchase_manager",
                &[
                    ("purchase-orders", "read"),
                    ("purchase-orders", "update"),
                    ("purchase-orders", "delete"),
                    ("purchase-orders", "approve"),
                    ("purchase-orders", "reject"),
                    ("purchase-receipts", "read"),
                    // 收货确认的真实端点是 POST /receipts/{id}/confirm，`confirm` 在动作关键字表内
                    // ⇒ 运行期键为 purchase-receipts:confirm；原先的 approve 行没有对应端点，
                    // 属悬空授权（点了也只会 403），改按真实键授予。
                    ("purchase-receipts", "confirm"),
                    // 让步接收＝对不合格品的放行决定，由采购经理与质量经理共签；
                    // 不授建单收货的采购员与仓管（职责分离，键语义见迁移
                    // business/m0090_grant_purchase_receipt_concession_rejudge.rs）
                    ("purchase-receipts", "concession"),
                    ("purchase-returns", "read"),
                    ("purchase-returns", "approve"),
                    ("purchase-returns", "reject"),
                    // 拒绝键与审批键成对授予同一批角色（同 sales 域口径，
                    // 成对性锁见 tests/contract_wave11_approve_reject_pairing_test.rs）
                    ("purchase-contracts", "read"),
                    ("purchase-contracts", "approve"),
                    ("purchase-contracts", "reject"),
                    ("purchase-prices", "read"),
                    ("purchase-prices", "approve"),
                    ("purchase-prices", "reject"),
                    ("sku-mappings", "read"),
                    ("sku-mappings", "update"),
                    ("sku-mappings", "delete"),
                    // 批量导入是对照表的成批建单通路，与建单同授予本岗
                    ("sku-mappings", "import"),
                    // 供应商商品/色号目录（对照表依赖的父级数据；采购经理审批可改不可建，
                    // 且保密：销售角色一律不授，故仅出现在采购域分组）
                    ("supplier-products", "read"),
                    ("supplier-products", "update"),
                    ("supplier-product-colors", "read"),
                    ("supplier-product-colors", "update"),
                    ("suppliers", "*"),
                    ("supplier-evaluations", "*"),
                    ("ap", "read"),
                    ("inventory", "read"),
                    // 对照表/转采购需读我方产品目录（我方内部主数据，非供应商保密信息）
                    ("products", "read"),
                    // 产品分类树（同为我方内部主数据；对照表维护页挂载即拉
                    // GET /product-categories/tree，缺码该页对采购岗恒 403 —— CI
                    // 矩阵 purchaser 真缺口，裁定 R-6：补真实种子而非前端降级隐藏）
                    ("product-categories", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "purchase_clerk",
                &[
                    ("purchase-orders", "read"),
                    ("purchase-orders", "create"),
                    ("purchase-orders", "update"),
                    ("purchase-receipts", "read"),
                    ("purchase-receipts", "create"),
                    // 建单与确认收货同线（与采购经理一致按真实键 confirm 授予）；
                    // 让步与改判仍不授本岗——见 quality 域的 concession/rejudge 分配。
                    ("purchase-receipts", "confirm"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-prices", "read"),
                    ("sku-mappings", "read"),
                    ("sku-mappings", "create"),
                    ("sku-mappings", "update"),
                    // 批量导入是对照表的成批建单通路，与建单同授予本岗
                    ("sku-mappings", "import"),
                    // 供应商商品/色号目录（采购员可建可改，对照表父级数据维护主力；保密域不授销售）
                    ("supplier-products", "read"),
                    ("supplier-products", "create"),
                    ("supplier-products", "update"),
                    ("supplier-product-colors", "read"),
                    ("supplier-product-colors", "create"),
                    ("supplier-product-colors", "update"),
                    // 对照表/转采购需读我方产品目录（我方内部主数据，非供应商保密信息）
                    ("products", "read"),
                    // 产品分类树：同 purchase_manager 口径（R-6）
                    ("product-categories", "read"),
                ],
            ),
            (
                "sourcing_specialist",
                &[
                    ("suppliers", "read"),
                    ("suppliers", "create"),
                    ("suppliers", "update"),
                    ("supplier-evaluations", "read"),
                    ("supplier-evaluations", "create"),
                    ("purchase-prices", "*"),
                    ("purchase-contracts", "read"),
                    ("sku-mappings", "*"),
                    // 供应商商品/色号目录（采购专员维护商品与色号目录，read/create/update）
                    ("supplier-products", "read"),
                    ("supplier-products", "create"),
                    ("supplier-products", "update"),
                    ("supplier-product-colors", "read"),
                    ("supplier-product-colors", "create"),
                    ("supplier-product-colors", "update"),
                    // 对照表/转采购需读我方产品目录（我方内部主数据，非供应商保密信息）
                    ("products", "read"),
                    // 产品分类树：同 purchase_manager 口径（R-6）
                    ("product-categories", "read"),
                ],
            ),
        ]
    }

    /// 库存仓储域角色权限定义（inventory_manager 库存经理 / warehouse_keeper 仓管员 /
    /// warehouse_manager 仓库经理）。
    fn inventory_role_resources() -> RoleResourceSlice {
        &[
            (
                "inventory_manager",
                &[
                    ("inventory", "*"),
                    ("stock", "*"),
                    // 匹号领域（GET /inventory/pieces）与成品布入库标签
                    // （GET /inventory/pieces/{id}/print）。键名 `pieces:read` / `pieces:print`
                    // 由 URL 段推导（middleware/permission.rs::extract_resource_info 对模块前缀
                    // inventory 取 segment4），与 `inventory:*` 不同源、不被其覆盖；
                    // 缺码则非 admin 的库存经理整页 403（本波 匹号选择器、 标签即此形态）。
                    // 存量库由迁移 m0069 同口径补授，e2e 侧由 global-setup 的
                    // SEED_ROLE_EXTRA_PERMISSIONS 同口径补授，三处清单一致性由
                    // tests/contract_wave7_piece_permission_grant_test.rs 双向钉。
                    ("pieces", "read"),
                    ("pieces", "print"),
                    ("piece-split", "*"),
                    ("transfers", "*"),
                    ("adjustments", "*"),
                    ("reservations", "*"),
                    ("counts", "*"),
                    ("batches", "*"),
                    ("stock-alerts", "*"),
                    ("products", "read"),
                    ("warehouses", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "warehouse_keeper",
                &[
                    ("inventory", "read"),
                    ("inventory", "create"),
                    ("inventory", "update"),
                    ("stock", "read"),
                    ("stock", "create"),
                    ("stock", "update"),
                    // 仓管收发发货需按四维选匹（/inventory/pieces）并打印成品布入库标签
                    ("pieces", "read"),
                    ("pieces", "print"),
                    ("transfers", "read"),
                    ("transfers", "create"),
                    ("counts", "read"),
                    ("counts", "create"),
                    ("products", "read"),
                    ("warehouses", "read"),
                ],
            ),
            (
                "warehouse_manager",
                &[
                    // 仓库经理按四维选匹（GET /inventory/pieces）并打印成品布入库标签
                    // （GET /inventory/pieces/{id}/print）。pieces 键由 URL 段推导、不被
                    // inventory:* 覆盖（同 inventory_manager 分组内注释），三通道同口径：
                    // 本分组与迁移 m0069 的 PRINT_ROLE_CODES/READ_ROLE_CODES、e2e 的
                    // SEED_ROLE_EXTRA_PERMISSIONS 集合一致性由
                    // tests/contract_wave7_piece_permission_grant_test.rs 双向钉。
                    // 此处仅授 pieces 两键——仓库经理其余库存/仓储操作权的矩阵面
                    // 维持现状（别岗权限不顺手扩）。
                    ("pieces", "read"),
                    ("pieces", "print"),
                ],
            ),
        ]
    }

    /// 生产核心域角色权限定义（production_manager / dyeing_master / finishing_master / lab_technician）。
    fn production_core_role_resources() -> RoleResourceSlice {
        &[
            (
                "production_manager",
                &[
                    ("production-orders", "*"),
                    ("dye-batches", "*"),
                    ("dye-recipes", "*"),
                    ("dye-batch-lifecycle-logs", "*"),
                    ("dye-batch-state-rules", "read"),
                    ("dye-batch-reworks", "*"),
                    ("dye-batch-operations", "*"),
                    ("greige-fabrics", "read"),
                    // 生产侧按匹跟踪（委外发料/收回、流转卡报工）需读匹号领域行
                    ("pieces", "read"),
                    ("lab-dip", "read"),
                    ("production-recipes", "*"),
                    ("process-routes", "*"),
                    ("flow-cards", "*"),
                    ("outsourcing-orders", "*"),
                    ("outsourcing-receipts", "*"),
                    ("business-modes", "read"),
                    // 生产经理要能查物料清单（/bom 页路由门是 boms:read，工单用料也以 BOM 为据），
                    // 而生产域此前没有任何 boms 授权 ⇒ 该页对本岗恒被守卫拦成 403。只授读。
                    ("boms", "read"),
                    ("mrp", "*"),
                    ("capacity", "*"),
                    ("scheduling", "*"),
                    ("material-shortage", "read"),
                ],
            ),
            (
                "dyeing_master",
                &[
                    ("dye-batches", "*"),
                    ("dye-batch-lifecycle-logs", "read"),
                    ("dye-batch-operations", "*"),
                    ("dye-batch-reworks", "*"),
                    ("dye-recipes", "read"),
                    ("dye-recipes", "create"),
                    ("production-recipes", "read"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                    ("flow-cards", "read"),
                    ("flow-cards", "update"),
                ],
            ),
            (
                "finishing_master",
                &[
                    ("production-orders", "read"),
                    ("production-orders", "create"),
                    ("production-orders", "update"),
                    ("dye-batches", "read"),
                    ("dye-batch-operations", "create"),
                    ("flow-cards", "read"),
                    ("flow-cards", "update"),
                    ("fabric-inspections", "read"),
                ],
            ),
            (
                "lab_technician",
                &[
                    ("dye-recipes", "*"),
                    ("lab-dip", "*"),
                    ("dye-batches", "read"),
                    ("color-cards", "*"),
                    ("color-prices", "read"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                ],
            ),
        ]
    }

    /// 生产扩展域角色权限定义（dye_recipe_master / greige_manager / chemical_manager / maintenance_supervisor）。
    fn production_extended_role_resources() -> RoleResourceSlice {
        // V15 P0-S18 新增：染色配方主管，含审批权限
        &[
            (
                "dye_recipe_master",
                &[
                    ("dye-recipes", "*"),
                    ("dye-recipes", "approve"),
                    ("dye-recipes", "audit"),
                    ("lab-dip", "*"),
                    ("dye-batches", "read"),
                    ("dye-batches", "update"),
                    ("production-recipes", "*"),
                    ("color-cards", "*"),
                    ("color-prices", "*"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "greige_manager",
                &[
                    ("greige-fabrics", "*"),
                    ("outsourcing-orders", "*"),
                    ("outsourcing-receipts", "*"),
                    ("outsourcing-vouchers", "read"),
                    ("inventory", "read"),
                    ("stock", "read"),
                    ("suppliers", "read"),
                    ("purchase-orders", "read"),
                ],
            ),
            (
                "chemical_manager",
                &[
                    ("chemicals", "*"),
                    ("chemical-categories", "*"),
                    ("chemical-lots", "*"),
                    ("chemical-requisitions", "*"),
                    ("purchase-orders", "read"),
                    ("inventory", "read"),
                    ("stock", "read"),
                ],
            ),
            (
                "maintenance_supervisor",
                &[
                    ("equipment", "*"),
                    ("maintenance-records", "*"),
                    ("production-orders", "read"),
                    ("energy-meters", "read"),
                    ("energy-consumptions", "read"),
                ],
            ),
        ]
    }

    /// 质量域角色权限定义（qc_manager 质量经理 / quality_inspector 质检员 / fabric_inspector 面料检验员）。
    fn quality_role_resources() -> RoleResourceSlice {
        &[
            (
                "qc_manager",
                &[
                    ("quality-inspections", "*"),
                    ("quality-issues", "*"),
                    ("quality-standards", "*"),
                    ("fabric-inspections", "*"),
                    ("fabric-defects", "*"),
                    // 质量经理既可放行让步，也可推翻既有质检结论（两枚键同授本岗，
                    // 但都不授采购员——避免"自判自放"）
                    ("purchase-receipts", "concession"),
                    ("purchase-receipts", "rejudge"),
                    ("dye-batches", "read"),
                    ("production-orders", "read"),
                    ("reports", "read"),
                    ("business-trace", "read"),
                ],
            ),
            (
                "quality_inspector",
                &[
                    ("quality-inspections", "*"),
                    ("quality-issues", "read"),
                    ("quality-issues", "create"),
                    // 复检改判由质检执行：不授任何采购岗，避免采购自判自改
                    ("purchase-receipts", "rejudge"),
                    ("dye-batches", "read"),
                    ("fabric-inspections", "read"),
                    ("fabric-inspections", "create"),
                    // 验布打卷产出成品布 → 标签面板读匹行并可打印
                    ("pieces", "read"),
                    ("pieces", "print"),
                ],
            ),
            (
                "fabric_inspector",
                &[
                    ("fabric-inspections", "*"),
                    ("fabric-defects", "*"),
                    ("dye-batches", "read"),
                    ("products", "read"),
                    ("quality-standards", "read"),
                    // 打卷员是本岗产出成品布的打印人（成品布入库标签）
                    ("pieces", "read"),
                    ("pieces", "print"),
                ],
            ),
        ]
    }

    /// 财务域角色权限定义（finance_manager / accountant / cashier / cost_accountant）。
    fn finance_role_resources() -> RoleResourceSlice {
        &[
            (
                "finance_manager",
                &[
                    ("vouchers", "*"),
                    ("subjects", "*"),
                    ("fixed-assets", "*"),
                    ("budgets", "*"),
                    ("cost-collections", "*"),
                    ("ar", "*"),
                    ("ap", "*"),
                    ("gl", "*"),
                    ("financial-analysis", "*"),
                    ("fund-management", "*"),
                    ("fund-transfers", "*"),
                    ("currencies", "*"),
                    ("exchange-rates", "*"),
                    ("ar-reconciliations", "*"),
                    ("accounting-periods", "*"),
                    ("wages", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "accountant",
                &[
                    ("vouchers", "*"),
                    ("subjects", "read"),
                    ("gl", "read"),
                    ("gl", "create"),
                    ("gl", "update"),
                    ("ar", "read"),
                    ("ar", "create"),
                    ("ap", "read"),
                    ("ap", "create"),
                    ("fixed-assets", "read"),
                    ("fixed-assets", "create"),
                    ("currencies", "read"),
                    ("exchange-rates", "read"),
                    ("ar-reconciliations", "read"),
                    ("ar-reconciliations", "create"),
                ],
            ),
            (
                "cashier",
                &[
                    ("fund-transfers", "read"),
                    ("fund-transfers", "create"),
                    ("fund-transfers", "update"),
                    ("fund-management", "read"),
                    ("fund-management", "create"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("vouchers", "read"),
                    ("vouchers", "create"),
                ],
            ),
            (
                "cost_accountant",
                &[
                    ("cost-collections", "*"),
                    ("vouchers", "read"),
                    ("vouchers", "create"),
                    ("dye-batches", "read"),
                    ("production-orders", "read"),
                    ("outsourcing-orders", "read"),
                    ("outsourcing-receipts", "read"),
                    ("wages", "read"),
                    ("energy-allocations", "read"),
                    ("gl", "read"),
                ],
            ),
        ]
    }

    /// CRM/物流/人力资源域角色权限定义（crm_manager / crm_rep / logistics_coordinator / customs_specialist / hr_manager / hr_specialist）。
    fn crm_logistics_hr_role_resources() -> RoleResourceSlice {
        &[
            (
                "crm_manager",
                &[
                    ("crm-leads", "*"),
                    ("crm-opportunities", "*"),
                    ("crm-customers", "*"),
                    ("customers", "*"),
                    ("customer-credits", "*"),
                    ("five-dimension", "*"),
                    ("sales-analysis", "*"),
                    ("reports", "read"),
                ],
            ),
            (
                "crm_rep",
                &[
                    ("crm-leads", "read"),
                    ("crm-leads", "create"),
                    ("crm-leads", "update"),
                    ("crm-opportunities", "read"),
                    ("crm-opportunities", "create"),
                    ("crm-opportunities", "update"),
                    ("crm-customers", "read"),
                    ("crm-customers", "create"),
                    ("customers", "read"),
                    ("customers", "create"),
                    // PII 按需揭示（POST /crm/customers/{id}/pii/reveal 的运行时键，
                    // 与 sales_rep/m0086/e2e SEED 三通道同口径；每次揭示强制留痕）
                    ("customers", "reveal"),
                ],
            ),
            // 客户服务：线索域四动作（read/create/update/delete），与
            // frontend/e2e/global-setup.ts SEED_ROLE_EXTRA_PERMISSIONS.customer_service
            // 的动作集逐码对齐（资源名取注册表/前端常量 `permissions.ts` 的规范码
            // crm-leads，e2e extras 用的是 /crm/leads 消歧前的运行时段 leads，
            // 两码同源一名之差，见本分组下方运行时资源段备注）。
            // 角色本体已由 role.rs 播种但矩阵此前无分组 ⇒ 角色级授权零行 ⇒ 登录后
            // 路由守卫送 /403；本分组同时让其经 SHELL_PERMISSIONS 拿到
            // dashboard/notifications 读，落地页恢复。
            // 刻意**不**附带 customers/crm-customers/商机等别岗码：data-scope-isolation
            // 用例的 403 必须来自行级归属校验（check_resource_owner，文案层判"无权限"），
            // 越界扩权会把拒绝来源层上移到 RBAC，破坏该安全断言的层级判据。
            // 运行时键同源：`/crm/leads` 的 URL 段派生已由
            // utils/path_utils.rs::resolve_module_prefixed_resource 消歧到本规范码
            // ("crm","leads") → crm-leads，与注册表登记名一致，故本授权行在 RBAC 匹配层
            // 真实命中（crm_manager/crm_rep 的同域既有码同时被复活）。
            (
                "customer_service",
                &[
                    ("crm-leads", "read"),
                    ("crm-leads", "create"),
                    ("crm-leads", "update"),
                    ("crm-leads", "delete"),
                ],
            ),
            (
                "logistics_coordinator",
                &[
                    ("logistics", "*"),
                    ("ship-orders", "*"),
                    ("orders", "read"),
                    ("inventory", "read"),
                    ("incoterms", "read"),
                    // 出口商检：物流协同跟踪出货需读商检单并打印报关随附单据（不建单、不改判结论）。
                    ("export-inspections", "read"),
                    ("export-inspections", "print"),
                    ("reports", "read"),
                ],
            ),
            (
                "customs_specialist",
                &[
                    ("logistics", "read"),
                    ("logistics", "create"),
                    ("logistics", "update"),
                    ("incoterms", "read"),
                    ("incoterms", "create"),
                    ("orders", "read"),
                    ("ship-orders", "read"),
                    // 出口商检：报关专员负责本域全生命周期——建单(POST)、登记/改判结论
                    // (PUT /{id}/result)、看单与打印商检/报关单据(GET 列表/详情/print)。
                    // 与同族 incoterms 授权面同口径；存量库由迁移 m0083、e2e 由 SEED 同补。
                    ("export-inspections", "read"),
                    ("export-inspections", "create"),
                    ("export-inspections", "update"),
                    ("export-inspections", "print"),
                ],
            ),
            (
                "hr_manager",
                &[
                    ("employees", "*"),
                    ("departments", "*"),
                    ("wages", "*"),
                    ("wage-rates", "*"),
                    ("wage-records", "*"),
                    ("users", "read"),
                    ("roles", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "hr_specialist",
                &[
                    ("employees", "read"),
                    ("employees", "create"),
                    ("employees", "update"),
                    ("departments", "read"),
                    ("wages", "read"),
                    ("wages", "create"),
                    ("wage-records", "read"),
                    ("wage-records", "create"),
                    ("users", "read"),
                ],
            ),
        ]
    }

    /// 其他域角色权限定义（safety_officer 安全员 / system_admin 系统管理员 / data_analyst 数据分析师 / admin_assistant 行政助理）。
    fn misc_role_resources() -> RoleResourceSlice {
        &[
            (
                "safety_officer",
                &[
                    ("safety-records", "*"),
                    ("environmental-records", "*"),
                    ("equipment", "read"),
                    ("maintenance-records", "read"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                    ("reports", "read"),
                    ("audit-logs", "read"),
                ],
            ),
            // 审计员：只读审计面，写法对齐上列 safety_officer 的只读审计授权
            // （audit-logs/reports 仅 read，零写码，最小授权）。
            // 该角色码被 utils/admin_checker.rs::AUDITOR_ROLE_CODE 与
            // handlers/audit_log_handler.rs（审计日志查询 admin/auditor 双角色深度
            // 防御）硬引用，必须是 roles 表在册岗位（由 role.rs 同码播种）而非
            // e2e 补建 fixture；缺册时闸门②点名判红，不会静默。
            // /audit-logs 的资源段是 seg3 直接资源（audit-logs 自身即注册码），
            // 本授权行运行期可命中，与 audit_log_handler 的角色深度防御双层一致。
            ("auditor", &[("audit-logs", "read"), ("reports", "read")]),
            (
                "system_admin",
                &[
                    ("users", "*"),
                    ("roles", "*"),
                    ("departments", "*"),
                    ("permissions", "*"),
                    ("field-permissions", "*"),
                    ("system-config", "*"),
                    ("audit-logs", "*"),
                    ("slow-queries", "*"),
                    ("print-templates", "*"),
                    ("data-import", "*"),
                    ("permissions-audit", "*"),
                    ("business-trace", "read"),
                ],
            ),
            (
                "data_analyst",
                &[
                    ("reports", "*"),
                    ("bi-analysis", "*"),
                    ("dashboard", "*"),
                    ("sales-analysis", "*"),
                    ("five-dimension", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("inventory", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                ],
            ),
            (
                "admin_assistant",
                &[
                    ("users", "read"),
                    ("users", "create"),
                    ("users", "update"),
                    ("departments", "read"),
                    ("oa-announcements", "*"),
                    ("notifications", "read"),
                    ("notifications", "create"),
                ],
            ),
        ]
    }

    /// 初始化默认角色互斥规则（SoD 职责分离，幂等）
    pub(crate) async fn create_default_role_conflicts(&self) -> Result<(), InitError> {
        // 幂等检查：表已有记录则跳过
        let existing_count = role_conflict::Entity::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("查询 role_conflicts 失败: {}", e)))?;
        if existing_count > 0 {
            return Ok(());
        }

        // 默认互斥角色对（role_a_code < role_b_code 字典序约定）
        // 来源：经典 SoD 职责分离 + 面料行业业务场景
        let conflicts: &[(&str, &str, &str, &str)] = &[
            // (role_a_code, role_b_code, conflict_type, description)
            (
                "accounting_clerk",
                "financial_manager",
                "sod",
                "制单与审核分离：会计员不可兼任财务经理",
            ),
            (
                "purchase_clerk",
                "purchase_manager",
                "sod",
                "采购制单与采购审批分离",
            ),
            (
                "purchase_manager",
                "finance_manager",
                "sod",
                "采购审批与付款审批分离",
            ),
            (
                "sales_clerk",
                "sales_manager",
                "sod",
                "销售制单与销售审批分离",
            ),
            (
                "production_worker",
                "quality_inspector",
                "sod",
                "生产执行与质量检验分离",
            ),
            (
                "production_manager",
                "quality_manager",
                "sod",
                "生产管理与质量管理分离",
            ),
            (
                "warehouse_keeper",
                "purchase_clerk",
                "sod",
                "入库与采购分离，防止自采自收",
            ),
            (
                "warehouse_keeper",
                "sales_clerk",
                "sod",
                "出库与销售分离，防止自销自发",
            ),
            (
                "admin",
                "quality_inspector",
                "sod",
                "管理员与质检员互斥（管理员不应直接执行质检）",
            ),
        ];

        let now = chrono::Utc::now();
        let models: Vec<role_conflict::ActiveModel> = conflicts
            .iter()
            .map(|(a, b, ctype, desc)| role_conflict::ActiveModel {
                id: Default::default(),
                role_a_code: Set(a.to_string()),
                role_b_code: Set(b.to_string()),
                conflict_type: Set(ctype.to_string()),
                description: Set(Some(desc.to_string())),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .collect();

        if let Err(e) = role_conflict::Entity::insert_many(models)
            .exec(self.db.as_ref())
            .await
        {
            warn!("批量创建角色互斥规则失败: {}, 可能部分已存在", e);
        }

        Ok(())
    }
}
