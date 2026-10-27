//! 公司档案服务
//!
//! 单例表 `company_profile`(id 恒 1)的读取与全量替换更新。职责边界:
//! - `get_profile`:取单例行;行缺席即数据完整性被破坏(迁移播种必然落行),以 500 点名,
//!   绝不静默造一个空档案兜底。
//! - `update_profile`:PUT 全量替换语义——提交的字段即档案终态,未提交的可选字段清空为
//!   NULL;`name` trim 后非空是硬校验;`logo` 非空时必须是 base64 可解码、PNG 魔数
//!   (89 50 4E 47)开头、解码后 ≤512KB 的裸串(无 data: 前缀)。校验失败以
//!   `AppError::validation_displayable` 中文可展示错误返回,成功与失败均落显式日志。
//! - `updated_by` 记录当前登录 user_id,`updated_at` 取写入时刻。
//!
//! 出参序列化键 = `models/company_profile::Model` 字段名(snake_case),不在本层再造第二套
//! DTO;公开投影(`CompanyProfilePublic`)在 handler 侧以硬投影结构体完成,本服务不裁剪。

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::NaiveDate;
use sea_orm::entity::prelude::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::company_profile;
use crate::utils::error::AppError;

/// 单例主键:全表唯一行,与迁移 `ck_company_profile_singleton CHECK (id = 1)` 同源同值。
pub const COMPANY_PROFILE_ID: i32 = 1;

/// logo 解码后字节上限(512KB)。公司抬头 logo 是小尺寸位图;超限即校验拒绝,
/// 不截断、不回落——防止巨型二进制借 base64 通道挤进单例 TEXT 列拖垮读路径。
pub const LOGO_MAX_BYTES: usize = 512 * 1024;

/// PNG 文件签名 8 字节(89 50 4E 47 0D 0A 1A 0A)。docx-rs 0.4 的 image_collector 仅
/// 收 PNG,故 logo 在写入口就按 PNG 校验,坏数据根本进不了库。这里取**完整 8 字节签名**
/// 而非仅前 4 字节,判据与前端 logo-file.ts 的 PNG_SIGNATURE 严格同源,防止只带前 4 字节
/// 的畸形/伪造头被 4 字节前缀误放。
const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// 打印页眉 logo 的固定缩放宽(像素):超宽者等比压到此宽,窄者不放大。
/// 抬头是辅助物料,不该抢正文版面;高度按 IHDR 原比例折算,docx-rs 内部换算 EMU。
pub const LETTERHEAD_LOGO_WIDTH_PX: u32 = 120;

/// 空白串归一为 None(trim 后为空视为未填),保证可空列不存留 " " 假值。
fn blank_to_none(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// 公司档案全量更新草稿:PUT 请求体经 handler 收敛后的终态值集(全字段覆盖语义)。
#[allow(dead_code, reason = "服务层入参结构,字段按列逐个消费")]
pub struct CompanyProfileDraft {
    pub name: String,
    pub short_name: Option<String>,
    pub credit_code: Option<String>,
    pub legal_representative: Option<String>,
    pub tax_registration_number: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub taxpayer_type: Option<String>,
    pub phone: Option<String>,
    pub fax: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub address: Option<String>,
    pub registered_capital: Option<Decimal>,
    pub establishment_date: Option<NaiveDate>,
    pub logo: Option<String>,
    pub description: Option<String>,
}

/// 解码并校验 logo base64:先归一(data URL 前缀剥离、剔除空白与换行),再按标准
/// base64 解码,解码结果必须以 PNG 魔数开头且 ≤512KB。归一的意义:历史遗留或调用方
/// 误带 `data:image/png;base64,` 前缀、MIME 变体前缀的合法 PNG 值,不应因前缀被判
/// "非 base64"或"非 PNG";而非 PNG 内容(伪 png)拒绝时点名**实际检测到的格式**,
/// 让"我上传的就是 png"的用户能看到真正被拒的原因。
/// 返回 PNG 原始字节,供调用方进一步取 IHDR 尺寸或嵌入 docx。
/// 失败一律 `validation_displayable` 中文可展示点名,调用方不得吞掉。
pub fn decode_logo_png(logo_base64: &str) -> Result<Vec<u8>, AppError> {
    let normalized = normalize_logo_base64(logo_base64);
    let bytes = BASE64_STANDARD.decode(&normalized).map_err(|e| {
        warn!(error = %e, "公司档案 logo base64 解码失败");
        AppError::validation_displayable("公司 logo 不是合法的 base64 编码,请重新上传")
    })?;
    if bytes.len() > LOGO_MAX_BYTES {
        warn!(
            size = bytes.len(),
            limit = LOGO_MAX_BYTES,
            "公司档案 logo 超出大小上限"
        );
        return Err(AppError::validation_displayable(format!(
            "公司 logo 解码后为 {} 字节,超过 512KB 上限,请压缩后重试",
            bytes.len()
        )));
    }
    if !bytes.starts_with(&PNG_MAGIC) {
        let detected = detect_image_format(&bytes);
        warn!(
            head = ?bytes.get(..4),
            detected,
            "公司档案 logo 非 PNG 魔数开头"
        );
        return Err(AppError::validation_displayable(format!(
            "公司 logo 文件内容不是 PNG 格式(实际检测为 {detected}),请另存为 PNG 后重新上传"
        )));
    }
    Ok(bytes)
}

/// 归一 logo base64:剥离 `data:<mime>;base64,` 前缀(MIME 变体如 image/x-png 同样
/// 覆盖),并剔除 ASCII 空白与换行(某些工具产出折行 base64)。幂等——裸串二次
/// 调用不变。
fn normalize_logo_base64(raw: &str) -> String {
    let without_prefix = match raw.split_once(";base64,") {
        Some((head, body)) if head.trim_start().starts_with("data:") => body,
        _ => raw,
    };
    without_prefix
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}

/// 按文件头魔数识别常见图片格式,用于拒绝文案点名真实格式(与前端 logo-file.ts
/// 判据同源);识别不出返回 "未知格式",不猜测。
fn detect_image_format(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "JPEG"
    } else if bytes.starts_with(b"GIF8") {
        "GIF"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "WebP"
    } else if bytes.starts_with(b"BM") {
        "BMP"
    } else if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        "ICO"
    } else {
        "未知格式"
    }
}

/// 从 PNG 字节读取 IHDR 宽高(像素)。签名 8 字节 + 块长 4 + 类型 4 后,宽占
/// [16..20]、高占 [20..24],均为大端 u32——这是 PNG 规范的固定布局。
/// 字节数不足或非 IHDR 类型名(魔数已由调用方校验)返回 None,由调用方决定失败形态。
pub fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || &png[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    if width == 0 || height == 0 {
        return None;
    }
    Some((width, height))
}

/// 公司档案服务(持有数据库连接,与 `PrintService` 同构按请求构造)
pub struct CompanyProfileService {
    db: Arc<DatabaseConnection>,
}

impl CompanyProfileService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 读取单例档案行。缺席 = 迁移播种行被人为删除,属数据完整性事故,500 点名,
    /// 不回落空档案(回落会把"表被人删空"伪装成"公司还没设置档案")。
    pub async fn get_profile(&self) -> Result<company_profile::Model, AppError> {
        company_profile::Entity::find_by_id(COMPANY_PROFILE_ID)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                warn!("company_profile 单例行缺席,迁移播种行被删除或未执行");
                AppError::internal(
                    "公司档案数据缺失(company_profile 单例行不存在,迁移播种异常或被误删)",
                )
            })
    }

    /// 全量替换更新单例档案(PUT 语义:提交的草稿即档案终态)。
    ///
    /// 校验:
    /// - `name` trim 后非空(且非全空白);
    /// - `logo` 非空时经 [`decode_logo_png`] 校验 base64 + PNG 魔数 + 512KB 上限,
    ///   空白串视为清除 logo;
    /// - 可空字符串列统一 trim 归一,空白落 NULL。
    ///
    /// 审计:成功与校验失败均落显式日志;落库时 `updated_by` = 当前 user_id、
    /// `updated_at` = 写入时刻,并回读返回更新后的整行。
    pub async fn update_profile(
        &self,
        draft: CompanyProfileDraft,
        user_id: i32,
    ) -> Result<company_profile::Model, AppError> {
        let name = draft.name.trim().to_string();
        if name.is_empty() {
            warn!(user_id, "公司档案更新被拒:公司名称为空");
            return Err(AppError::validation_displayable("公司名称不能为空"));
        }
        let logo = match draft.logo.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(raw) => {
                // 解码校验通过才落库；落库前归一为裸 base64（剥前缀、去折行），
                // 库里永远只存在合法 PNG 裸串——带前缀的历史值经此写口一次即被纠正。
                decode_logo_png(raw)?;
                Some(normalize_logo_base64(raw))
            }
        };

        let existing = self.get_profile().await?;
        let logo_len = logo.as_ref().map(String::len).unwrap_or(0);
        info!(
            user_id,
            name = %name,
            logo_bytes = logo_len,
            "更新公司档案"
        );

        let mut active: company_profile::ActiveModel = existing.into_active_model();
        active.name = Set(name);
        active.short_name = Set(blank_to_none(draft.short_name));
        active.credit_code = Set(blank_to_none(draft.credit_code));
        active.legal_representative = Set(blank_to_none(draft.legal_representative));
        active.tax_registration_number = Set(blank_to_none(draft.tax_registration_number));
        active.bank_name = Set(blank_to_none(draft.bank_name));
        active.bank_account = Set(blank_to_none(draft.bank_account));
        active.taxpayer_type = Set(blank_to_none(draft.taxpayer_type));
        active.phone = Set(blank_to_none(draft.phone));
        active.fax = Set(blank_to_none(draft.fax));
        active.email = Set(blank_to_none(draft.email));
        active.website = Set(blank_to_none(draft.website));
        active.address = Set(blank_to_none(draft.address));
        active.registered_capital = Set(draft.registered_capital);
        active.establishment_date = Set(draft.establishment_date);
        active.logo = Set(logo);
        active.description = Set(draft.description.filter(|s| !s.trim().is_empty()));
        active.updated_at = Set(chrono::Utc::now().into());
        active.updated_by = Set(user_id);
        let model = active.update(&*self.db).await?;
        info!(user_id, profile_id = model.id, "公司档案更新成功");
        Ok(model)
    }
}
