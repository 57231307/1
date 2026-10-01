use crate::models::api_key::{self, ActiveModel as ApiKeyActiveModel, Entity as ApiKey};
use crate::models::user;
use crate::utils::cache::{AppCache, Cache};
use crate::utils::error::AppError;
use crate::utils::random;
use chrono::{DateTime, Utc};
use sea_orm::FromQueryResult;
use sea_orm::*;
use std::time::Duration;

crate::define_service!(ApiKeyService);

/// API Key 黑名单缓存 TTL（秒）
/// 漏洞 #5 修复：撤销后 key_hash 写入 AppCache.token_blacklist，；TTL 与 key 有效期对齐（最长 7 天）。TTL 过后认为黑名单自动失效。；典型业务场景：用户撤销 → 立即生效 → TTL 内强制吊销。
const API_KEY_BLACKLIST_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// 黑名单缓存键前缀
pub const API_KEY_BLACKLIST_PREFIX: &str = "apikey:revoked:";

/// 读侧视图对象：`api_keys` 全列 + LEFT JOIN `users` 派生列 `created_by_name`。
///
/// 派生列声明 `Option<String>` 是因为 LEFT JOIN 在 `created_by` 为 NULL（历史数据）
/// 或用户行缺失（删档/悬挂引用）时产 NULL——忠实反映可空性，禁止以 id、空串或
/// "未知" 等拼装假名填充（范式来源：`services/custom_order_aftersales_service.rs`
/// 的 `customer_name` 富化链路，提交 13f6bd09）。
#[derive(Debug, Clone, PartialEq, FromQueryResult)]
pub struct ApiKeyWithCreator {
    pub id: i32,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub permissions: Option<String>,
    pub rate_limit_per_minute: i32,
    pub last_used_at: Option<DateTime<Utc>>,
    /// NULL = 永不过期（列真值，出参不得塌成空串）
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_by: Option<i32>,
    /// NULL = 未填描述（列真值，出参不得塌成空串）
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// `users.username`：由 `select_with_creator()` 的 LEFT JOIN 提供唯一来源
    pub created_by_name: Option<String>,
}

/// `api_keys.created_by` → `users.id` 的关联定义（就地构造）。
///
/// `models/api_key.rs::Relation` 是空枚举（模型文件不在本次改动范围），按本仓
/// `services/inv/inventory_move.rs` 先例用 `Entity::belongs_to` 就地构造，不改实体。
fn creator_relation() -> RelationDef {
    ApiKey::belongs_to(user::Entity)
        .from(api_key::Column::CreatedBy)
        .to(user::Column::Id)
        .into()
}

/// api_keys 读侧唯一富化链路：全列 + `column_as(users.username, "created_by_name")`
/// + `JoinType::LeftJoin`。列表 / 详情 / 写后回读必须共用本函数，保证三端同源。
pub fn select_with_creator() -> Select<api_key::Entity> {
    ApiKey::find()
        .column_as(user::Column::Username, "created_by_name")
        .join(JoinType::LeftJoin, creator_relation())
}

/// 更新 API 密钥参数对象（批次 413 技术债务清理：引入参数对象消除 update_api_key 的 too_many_arguments 警告。；聚合更新 API 密钥所需的全部可选字段，避免函数签名携带 7 个参数。）
#[derive(Debug, Clone)]
pub struct UpdateApiKeyPayload {
    /// API 密钥 ID
    pub id: i32,
    /// 密钥名称（None=保持原值）
    pub name: Option<String>,
    /// 权限列表（JSON 字符串）
    pub permissions: Option<String>,
    /// 每分钟速率限制
    pub rate_limit_per_minute: Option<i32>,
    /// 过期时间（None=保持原值，Some(None)=永不过期，Some(Some(dt))=指定时间）
    pub expires_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    /// 是否启用
    pub is_active: Option<bool>,
    /// 描述（三态：None=保持原值，Some(None)=清空为 NULL，Some(Some(v))=覆盖）
    pub description: Option<Option<String>>,
}

impl ApiKeyService {
    /// 生成新的 API 密钥
    pub fn generate_api_key() -> String {
        // 32 位密码学安全随机串（4.9 修复：原 fastrand 可预测，改用 OsRng）
        let key = random::secure_random_alphanumeric(32);
        format!("bx_{}", key)
    }

    /// 哈希 API 密钥
    pub fn hash_api_key(key: &str) -> String {
        crate::utils::hash::sha256_hex(key.as_bytes())
    }

    /// 创建 API 密钥（批次 112 P1-9：新增 created_by 参数，注入真实创建者 user_id（原表无此列，handler 传 0 占位））
    ///
    /// 契约收口（本轮）：
    /// - `description` 真实落库（此前创建链路根本不接收该字段，用户填了被静默丢弃）；
    /// - `expires_at` 改为接收 handler 精确解析后的绝对时刻（原 `expires_days` 由
    ///   `now + days` 反算，会把用户选的精确到期时间改写成一个近似值，且解析失败
    ///   被 `.ok()` 吞成"永不过期"）。NULL = 永不过期。
    pub async fn create_api_key(
        &self,
        name: &str,
        description: Option<&str>,
        permissions: Option<&str>,
        rate_limit: i32,
        expires_at: Option<DateTime<Utc>>,
        created_by: i32,
    ) -> Result<(api_key::Model, String), AppError> {
        let plain_key = Self::generate_api_key();
        let key_hash = Self::hash_api_key(&plain_key);
        let key_prefix = plain_key[..8].to_string();

        let now = Utc::now();

        let active_model = ApiKeyActiveModel {
            name: Set(name.to_string()),
            key_hash: Set(key_hash),
            key_prefix: Set(key_prefix),
            permissions: Set(permissions.map(|s| s.to_string())),
            rate_limit_per_minute: Set(rate_limit),
            last_used_at: Set(None),
            expires_at: Set(expires_at),
            is_active: Set(true),
            // 批次 112 P1-9：持久化真实创建者 user_id
            created_by: Set(Some(created_by)),
            // 契约收口：创建即写入真实描述（NULL = 用户未填）
            description: Set(description.map(|s| s.to_string())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let model = active_model.insert(self.db.as_ref()).await?;
        Ok((model, plain_key))
    }

    /// 撤销 API 密钥
    /// 漏洞 #5 修复：撤销时同时将 `key_hash` 加入 `AppCache.token_blacklist` 缓存。；历史问题：原实现仅设置 `is_active = false`，旧的明文 API Key；（若被攻击者截获）在撤销后仍能继续使用。；修复策略：撤销时通过 `key_hash` 写入黑名单，未来 API Key 认证中间件；可通过 [`Self::is_api_key_revoked`] 检查是否已撤销。；TTL 7 天后自动失效（与典型 API Key 生命周期对齐）。
    pub async fn revoke_api_key(&self, id: i32, cache: Option<&AppCache>) -> Result<(), AppError> {
        let key = ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("API 密钥不存在"))?;

        let mut active_model: ApiKeyActiveModel = key.clone().into();
        active_model.is_active = Set(false);
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        // 漏洞 #5 修复：将 key_hash 加入黑名单缓存，TTL 7 天
        if let Some(cache) = cache {
            let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key.key_hash);
            cache.get_token_blacklist().set(
                blacklist_key,
                true,
                Some(Duration::from_secs(API_KEY_BLACKLIST_TTL_SECS)),
            );
            tracing::info!(
                "API 密钥已撤销并加入黑名单：id={}, key_prefix={}",
                id,
                key.key_prefix
            );
        } else {
            // 调用方未传入 cache：仅 DB 标记 is_active=false，黑名单失效
            // 保留 warn 日志便于运维发现未接管的调用方
            tracing::warn!("API 密钥撤销时未传入 AppCache，黑名单失效：id={}", id);
        }

        Ok(())
    }

    /// 按 ID 获取 API 密钥（批次 91 P0-1）
    pub async fn get_api_key_by_id(&self, id: i32) -> Result<Option<api_key::Model>, AppError> {
        ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 按 ID 回读「密钥行 + 真实创建者用户名」（`created_by_name`）。
    ///
    /// 列表（`select_with_creator()` 直接分页）、详情、以及 create/update/regenerate
    /// 写后出参必须全部经本链路，`created_by_name` 只允许来自 LEFT JOIN 真值；
    /// 用户行缺失时如实为 NULL（禁止在构造点填空串或拼装名）。
    pub async fn get_api_key_with_creator(
        &self,
        id: i32,
    ) -> Result<Option<ApiKeyWithCreator>, AppError> {
        select_with_creator()
            .filter(api_key::Column::Id.eq(id))
            .into_model::<ApiKeyWithCreator>()
            .one(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 更新 API 密钥（批次 91 P0-1）
    /// 仅更新传入的字段，未传入的字段保持不变。；批次 158 v11 真实接入：新增 description 参数持久化（原 #[allow(dead_code)] 移除）；批次 413 技术债务清理：签名从 7 参数改为单一参数对象 `UpdateApiKeyPayload`，；消除 `clippy::too_many_arguments` 警告。；本轮契约收口：`expires_at` / `description` 均为 `Option<Option<T>>` 三态——；键缺席=保持原值、显式 null=落 NULL（永不过期 / 清空描述）、有值=覆盖。
    pub async fn update_api_key(
        &self,
        payload: UpdateApiKeyPayload,
    ) -> Result<api_key::Model, AppError> {
        let key = ApiKey::find_by_id(payload.id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("API 密钥不存在"))?;

        let mut active_model: ApiKeyActiveModel = key.into();
        if let Some(name) = payload.name {
            active_model.name = Set(name);
        }
        if let Some(permissions) = payload.permissions {
            active_model.permissions = Set(Some(permissions));
        }
        if let Some(rate_limit) = payload.rate_limit_per_minute {
            active_model.rate_limit_per_minute = Set(rate_limit);
        }
        if let Some(expires_at) = payload.expires_at {
            active_model.expires_at = Set(expires_at);
        }
        if let Some(is_active) = payload.is_active {
            active_model.is_active = Set(is_active);
        }
        // 三态清空语义（对齐 RFC 7386 与本仓 double_option 范式）：
        // 外层 None=键缺席不动列；Some(None)=显式 null → 落 NULL；Some(Some(v))=覆盖
        if let Some(description) = payload.description {
            active_model.description = Set(description);
        }
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 重新生成 API 密钥（批次 91 P0-1）（生成新的明文密钥 + 哈希，旧 key_hash 加入黑名单。；返回 (更新后的 model, 新明文密钥)。）
    pub async fn regenerate_api_key(
        &self,
        id: i32,
        cache: Option<&AppCache>,
        // 批次 112 P1-9：新增 regenerated_by 参数，更新 created_by 为重新生成操作者
        regenerated_by: i32,
    ) -> Result<(api_key::Model, String), AppError> {
        let key = ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("API 密钥不存在"))?;

        // 旧 key_hash 加入黑名单
        if let Some(cache) = cache {
            let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key.key_hash);
            cache.get_token_blacklist().set(
                blacklist_key,
                true,
                Some(Duration::from_secs(API_KEY_BLACKLIST_TTL_SECS)),
            );
        }

        // 生成新密钥
        let plain_key = Self::generate_api_key();
        let key_hash = Self::hash_api_key(&plain_key);
        let key_prefix = plain_key[..8].to_string();

        let mut active_model: ApiKeyActiveModel = key.into();
        active_model.key_hash = Set(key_hash);
        active_model.key_prefix = Set(key_prefix);
        active_model.is_active = Set(true);
        // 批次 112 P1-9：重新生成时更新 created_by 为操作者（语义：新密钥的创建者）
        active_model.created_by = Set(Some(regenerated_by));
        active_model.updated_at = Set(Utc::now());

        let model = active_model.update(self.db.as_ref()).await?;
        Ok((model, plain_key))
    }

    /// 检查 API Key 是否已被撤销（漏洞 #5 修复）
    /// 流程：1. 计算明文 key 的 SHA-256 哈希；2. 检查 `AppCache.token_blacklist` 中是否存在 `<prefix><key_hash>` 条目；未来 API Key 认证中间件应在每次校验 key 前调用此方法。；# 参数；`cache`: 应用全局缓存；`plain_key`: 明文 API Key（含 `bx_` 前缀）；# 返回；`true`: 已撤销（拒绝使用）；`false`: 未撤销或黑名单已过期
    pub fn is_api_key_revoked(cache: &AppCache, plain_key: &str) -> bool {
        let key_hash = Self::hash_api_key(plain_key);
        let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key_hash);
        // Cache::get 返回 Option<V>（已 Clone），无需 .copied()
        cache
            .get_token_blacklist()
            .get(&blacklist_key)
            .unwrap_or(false)
    }
}
