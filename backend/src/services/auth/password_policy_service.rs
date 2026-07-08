//! 密码策略服务（P4-2 安全加固）
//!
//! 在现有 `utils/password_validator.rs` 基础上提供：
//! 1. 密码强度校验：复用 PasswordPolicy 算法
//! 2. 密码历史：禁止复用最近 N 次密码（批次 158 v11 真实接入 DB 持久化）
//! 3. 密码过期：强制 90 天轮换（批次 198 P0-2 真实接入 login 流程）
//!
//! 账户锁定策略由 auth_handler::login 基于 log_login 表独立实现（持久化 + per-IP/per-username 双阈值），
//! 本服务不重复实现锁定逻辑。

use crate::models::password_history;
use crate::utils::password_validator::{PasswordPolicy, PasswordValidationResult};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 密码历史（每用户最多保留 N 次历史密码，N 由 PasswordPolicyService.history_capacity 控制）
#[derive(Debug, Clone, Default)]
pub struct PasswordHistory {
    /// 历史上使用过的密码哈希
    pub history: VecDeque<String>,
    /// 上限
    pub capacity: usize,
}

impl PasswordHistory {
    /// 创建指定容量的历史
    pub fn new(capacity: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// 添加新密码到历史
    pub fn push(&mut self, hash: String) {
        if self.history.len() >= self.capacity {
            self.history.pop_front();
        }
        self.history.push_back(hash);
    }

    /// 检查密码是否在历史中
    pub fn contains(&self, hash: &str) -> bool {
        self.history.iter().any(|h| h == hash)
    }
}

/// 密码策略服务（用户级密码强度 + 历史 + 过期）
#[derive(Clone)]
pub struct PasswordPolicyService {
    /// 全局默认策略
    pub default_policy: Arc<RwLock<PasswordPolicy>>,
    /// 密码历史容量
    pub history_capacity: usize,
    /// 密码有效期（天），None = 永不过期（批次 198 P0-2 真实接入 login）
    pub max_age_days: Option<i64>,
}

impl Default for PasswordPolicyService {
    fn default() -> Self {
        Self {
            default_policy: Arc::new(RwLock::new(PasswordPolicy::default())),
            history_capacity: 5,
            max_age_days: Some(90),
        }
    }
}

impl PasswordPolicyService {
    /// 创建默认策略服务
    pub fn new() -> Self {
        Self::default()
    }

    /// 校验密码强度（不检查历史）
    pub async fn validate(&self, password: &str) -> PasswordValidationResult {
        let policy = self.default_policy.read().await.clone();
        crate::utils::password_validator::validate_password_with_policy(password, &policy)
    }

    /// 校验密码 + 排除历史
    ///
    /// 批次 158 v11 真实接入：由 change_password handler 调用
    pub async fn validate_with_history(
        &self,
        password: &str,
        new_hash: &str,
        history: &PasswordHistory,
    ) -> PasswordValidationResult {
        let mut result = self.validate(password).await;
        if history.contains(new_hash) {
            result.is_valid = false;
            result.errors.push("密码不能与最近使用过的密码相同".to_string());
        }
        result
    }

    /// 检查密码是否过期（批次 198 P0-2 真实接入 login 流程）
    ///
    /// 由 auth_handler::login 在认证成功后调用，
    /// 根据 users.password_changed_at 字段判断密码是否超过 max_age_days。
    pub fn is_expired(&self, last_changed: chrono::DateTime<chrono::Utc>) -> bool {
        match self.max_age_days {
            None => false,
            Some(days) => {
                let now = chrono::Utc::now();
                now.signed_duration_since(last_changed).num_days() > days
            }
        }
    }

    /// 从数据库加载用户密码历史（批次 158 v11 真实接入）
    pub async fn load_history_from_db(
        &self,
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<PasswordHistory, sea_orm::DbErr> {
        let rows = password_history::Entity::find()
            .filter(password_history::Column::UserId.eq(user_id))
            .order_by_desc(password_history::Column::CreatedAt)
            .limit(self.history_capacity as u64)
            .all(db)
            .await?;
        let mut history = PasswordHistory::new(self.history_capacity);
        // 按时间正序填入（旧→新），保持 push 语义
        for row in rows.into_iter().rev() {
            history.push(row.password_hash);
        }
        Ok(history)
    }

    /// 将旧密码哈希写入数据库历史表（批次 158 v11 真实接入）
    pub async fn save_to_db(
        &self,
        db: &DatabaseConnection,
        user_id: i32,
        password_hash: String,
    ) -> Result<(), sea_orm::DbErr> {
        let now = chrono::Utc::now();
        let active = password_history::ActiveModel {
            user_id: sea_orm::Set(user_id),
            password_hash: sea_orm::Set(password_hash),
            created_at: sea_orm::Set(now),
            ..Default::default()
        };
        active.insert(db).await?;
        Ok(())
    }
}

/// 检查常见弱密码集合（独立可复用）
pub fn is_common_password(password: &str) -> bool {
    let lower = password.to_lowercase();
    const COMMON: &[&str] = &[
        "password", "123456", "qwerty", "admin", "root",
        "letmein", "welcome", "monkey", "dragon", "111111",
        "000000", "abc123", "admin123", "passw0rd", "iloveyou",
    ];
    COMMON.iter().any(|c| lower.contains(c))
}

/// 检查密码是否包含用户名片段
pub fn contains_username_fragment(password: &str, username: &str) -> bool {
    if username.is_empty() {
        return false;
    }
    let lower_pwd = password.to_lowercase();
    let lower_user = username.to_lowercase();
    lower_pwd.contains(&lower_user)
}

/// 生成密码强度反馈（多语言友好版）
pub fn strength_feedback_zh(result: &PasswordValidationResult) -> String {
    if result.is_valid {
        format!(
            "密码强度：{}（得分 {}），符合安全要求",
            result.strength.description(),
            result.strength.score()
        )
    } else {
        format!(
            "密码强度不足（{}，{} 分）：{}",
            result.strength.description(),
            result.strength.score(),
            result.errors.join("；")
        )
    }
}

/// 构建默认 HashSet 用于快速密码查询（运行时使用）
pub fn build_password_blacklist() -> HashSet<String> {
    const BLACKLIST: &[&str] = &[
        "password", "123456", "qwerty", "admin", "root", "toor",
        "letmein", "welcome", "111111", "000000", "abc123",
    ];
    BLACKLIST.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn 测试_密码策略_强密码通过() {
        let svc = PasswordPolicyService::new();
        let result = svc.validate("MyP@ssw0rd_2026!").await;
        assert!(result.is_valid, "强密码应通过：{:?}", result.errors);
    }

    #[tokio::test]
    async fn 测试_密码策略_弱密码拒绝() {
        let svc = PasswordPolicyService::new();
        let result = svc.validate("123").await;
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn 测试_密码历史_防止复用() {
        let mut history = PasswordHistory::new(5);
        history.push("hash1".to_string());
        history.push("hash2".to_string());
        assert!(history.contains("hash1"));
        assert!(!history.contains("hash3"));
    }

    #[test]
    fn 测试_密码历史_容量上限() {
        let mut history = PasswordHistory::new(3);
        history.push("h1".to_string());
        history.push("h2".to_string());
        history.push("h3".to_string());
        history.push("h4".to_string());
        assert!(!history.contains("h1")); // 被淘汰
        assert!(history.contains("h4"));
    }

    #[test]
    fn 测试_密码过期() {
        let svc = PasswordPolicyService::new();
        let old = chrono::Utc::now() - chrono::Duration::days(100);
        assert!(svc.is_expired(old));
        let recent = chrono::Utc::now() - chrono::Duration::days(30);
        assert!(!svc.is_expired(recent));
    }

    #[test]
    fn 测试_常见密码识别() {
        assert!(is_common_password("Password"));
        assert!(is_common_password("123456"));
        assert!(!is_common_password("X7#mK9pQ@2vL"));
    }

    #[test]
    fn 测试_密码包含用户名片段() {
        assert!(contains_username_fragment("zhangsan@2026", "zhangsan"));
        assert!(!contains_username_fragment("X7#mK9pQ@2vL", "zhangsan"));
    }

    #[test]
    fn 测试_密码黑名单构建() {
        let blacklist = build_password_blacklist();
        assert!(blacklist.contains("password"));
        assert!(blacklist.contains("123456"));
        assert!(!blacklist.contains("X7#mK9pQ@2vL"));
    }
}
