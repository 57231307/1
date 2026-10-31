//! 扫码入库的解析定位与登记腿
//!
//! 承接 services 下的 inbound_qr_code 的定长切分结果，把二维码里的外部编码换成本系统
//! 内部身份，然后落一行库存匹。三段职责依次是定位，把款号与染厂色号与库位编码解析成
//! 产品与内部色号与仓库。查重，同一款号同缸号同匹号的染色匹已经入过库就拒绝再来一遍。
//! 登记，写且只写一行库存匹。
//!
//! 外部编码到内部身份的翻译只走供应商 SKU 对照表这一条路，因为染厂色号是染厂侧的编码，
//! 本系统的 product_colors.color_no 是内部编码，两者之间没有可推导关系，靠猜或取第一条
//! 都会把布入到别的颜色名下。
//!
//! 保密口径，本文件的现场文案只点名操作工自己扫进来的东西，即二维码里已校验为 ASCII 字母
//! 数字的四段与库位编码，不点名任何记录 ID、表名与列名。需要记录 ID 才能定位的异常一律
//! 只进 tracing 日志，文案仍说清下一步该做什么。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter,
    QuerySelect, RelationTrait, Set,
};

use crate::models::status::purchase_inventory::inventory_piece as piece_status;
use crate::models::{
    inventory_piece, location, product, product_color, product_supplier_mapping,
    supplier_product_color, warehouse,
};
use crate::services::inbound_qr_code::{ParsedRollQr, QrRejection, parse_inbound_qr};
use crate::services::piece_domain_service::{
    PIECE_TYPE_DYED, validate_piece_measured_value, validate_warehouse_for_piece_type,
};
use crate::utils::error::AppError;

/// 一次扫码入库的请求值，全部字段来自操作工的两次扫码与一次米数录入。
pub struct ScanInboundCommand<'a> {
    /// 面料二维码原文，整串原样接收，落库时也按这一整串写条码列
    pub raw_qr: &'a str,
    /// 先扫到的库位编码，对应库位档案里的库位编码列
    pub position_no: &'a str,
    /// 供应商 ID，给出时染厂色号只在该供应商的对照范围内定位
    pub supplier_id: Option<i32>,
    /// 本匹长度（米），请求可缺值，缺值在登记腿显式拒绝，不兜底
    pub length: Option<rust_decimal::Decimal>,
    /// 本匹重量，可空，空表示未实测
    pub weight: Option<rust_decimal::Decimal>,
    /// 操作人用户 ID，写进匹行的创建人列
    pub operator_id: i32,
}

/// 定位成功后交付的内部身份，登记腿按这些值落库。
pub struct ResolvedRollIdentity {
    /// 我方产品 ID
    pub product_id: i32,
    /// 内部色号 ID
    pub product_color_id: i32,
    /// 内部色号编码，落进匹行的色号列
    pub internal_color_no: String,
    /// 库位 ID
    pub location_id: i32,
    /// 库位所属仓库 ID
    pub warehouse_id: i32,
}

/// 定位与查重的拒绝原因，一变体一理由，调用方据此出文案，不允许折叠成一条。
///
/// 每个变体自带的材料只允许是操作工自己扫进来的编码值与先前入库的仓库编码与入库时间。
/// 需要点名记录 ID 的异常不带 ID，改由 to_app_error 给出仍可行动的文案，
/// 真实细节由产生点写日志。
pub enum ResolveRejection {
    /// 二维码里的款号在产品档案里一条都找不到
    ProductNotFound { product_code: String },
    /// 二维码里的款号在产品档案里命中多条，无法判定是哪一款
    ProductAmbiguous { product_code: String },
    /// 染厂色号在 SKU 对照表里没有任何启用中的对照
    NoColorMapping {
        product_code: String,
        external_color_no: String,
        supplier_scoped: bool,
    },
    /// 有对照记录，但那几条对照没有指定内部色号
    MappingWithoutInternalColor {
        product_code: String,
        external_color_no: String,
    },
    /// 染厂色号对照到多个不同的内部色号，无法选定一个
    AmbiguousColorMapping {
        product_code: String,
        external_color_no: String,
        candidate_count: usize,
    },
    /// 对照记录指向的内部色号行已经不存在
    InternalColorRowMissing,
    /// 对照记录指向的内部色号属于另一个产品
    InternalColorProductMismatch,
    /// 扫到的库位编码在库位档案里一条都没有
    PositionNotFound { position_no: String },
    /// 扫到的库位编码在多个仓库里都登记了，无法判定入哪个仓
    PositionAmbiguous { position_no: String },
    /// 同款号同缸号同匹号的染色匹已经存在，本次是重复扫码
    AlreadyInbound {
        duplicate_count: usize,
        warehouse_code: Option<String>,
        warehouse_in_at: Option<chrono::DateTime<chrono::Utc>>,
    },
    /// 匹行已存在但扫到的库位与先前记录的库位不同，属挪位诉求而非重复入库
    PositionMismatchOnExisting {
        existing_position_no: Option<String>,
        scanned_position_no: String,
        warehouse_code: Option<String>,
        warehouse_in_at: Option<chrono::DateTime<chrono::Utc>>,
    },
}

/// 扫码入库定位腿的失败面，要么是可外显的定位拒绝，要么是数据库错误。
///
/// 为什么要两个面分开，定位拒绝是操作工能自己修的事，数据库错误是系统故障，
/// 把后者折成前者会让操作工白重扫一遍，而真正该报的是连接或查询失败。
pub enum ResolveFailure {
    /// 定位或查重得出的业务拒绝
    Rejection(ResolveRejection),
    /// 只读查询本身的失败，原样保留 AppError 的分类与日志
    Db(AppError),
}

impl From<ResolveRejection> for ResolveFailure {
    /// 把定位拒绝装进失败面。
    ///
    /// 做什么，一个无损包装，不改文案也不改判定。
    /// 调用方，本模块各定位函数里用 into 把拒绝原因装进失败面的写法。
    /// 入参，rej，一个定位或查重的拒绝原因。
    /// 出参消费方，register_scanned_roll 用问号运算符取走后转成 AppError 出参。
    /// 存什么，不存任何东西。
    /// 存哪里，无落库。
    fn from(rej: ResolveRejection) -> Self {
        ResolveFailure::Rejection(rej)
    }
}

impl From<sea_orm::DbErr> for ResolveFailure {
    /// 把数据库错误装进失败面的另一侧。
    ///
    /// 做什么，转交 AppError 既有的 DbErr 分类，不在此处另写一套判定。
    /// 调用方，本模块各定位函数里对查询结果用问号运算符隐式转换的路径。
    /// 入参，err，sea-orm 抛出的连接、执行或查询错误。
    /// 出参消费方，register_scanned_roll 用问号运算符取走后原样出参。
    /// 存什么，不存任何东西，AppError 的转换内部已记日志。
    /// 存哪里，无落库。
    fn from(err: sea_orm::DbErr) -> Self {
        ResolveFailure::Db(AppError::from(err))
    }
}

impl From<ResolveFailure> for AppError {
    /// 把定位腿的失败面收成 AppError。
    ///
    /// 做什么，业务拒绝走文案翻译，数据库错误原样保留。
    /// 调用方，register_scanned_roll 内对定位结果用问号运算符隐式转换的路径。
    /// 入参，failure，定位或查重的失败面。
    /// 出参消费方，axum 的 IntoResponse 出统一错误信封给扫码入库端点。
    /// 存什么，不存任何东西。
    /// 存哪里，无落库。
    fn from(failure: ResolveFailure) -> Self {
        match failure {
            ResolveFailure::Rejection(rej) => rej.to_app_error(),
            ResolveFailure::Db(err) => err,
        }
    }
}

impl ResolveRejection {
    /// 把定位与查重的拒绝原因翻成 AppError，逐条决定文案外显还是脱敏。
    ///
    /// 做什么，只做拒绝原因到文案的映射，判据是文案里有没有点名记录 ID 一类内部标识，
    /// 本函数不查库也不记日志，材料在产生点已经带齐。
    /// 调用方，From<ResolveFailure> for AppError 这一条路径。
    /// 入参，self，一个定位或查重的拒绝原因。
    /// 出参消费方，扫码入库端点的错误响应体 message 字段。
    /// 存什么，不存任何东西。
    /// 存哪里，无落库。
    fn to_app_error(self) -> AppError {
        match self {
            ResolveRejection::ProductNotFound { product_code } => {
                AppError::business_displayable(format!(
                    "二维码款号 {product_code} 在本系统里找不到对应的产品，请先到产品档案确认该款是否已建档，建好档再扫"
                ))
            }
            ResolveRejection::ProductAmbiguous { product_code } => {
                AppError::business_displayable(format!(
                    "二维码款号 {product_code} 对应到多条产品记录，系统不会任选其中一条入库，请让产品主数据维护人把重复编码收敛成一条"
                ))
            }
            ResolveRejection::NoColorMapping {
                product_code,
                external_color_no,
                supplier_scoped,
            } => {
                let scope = if supplier_scoped {
                    "，本次已限定在选定的供应商范围内查找"
                } else {
                    "，本次已在该款全部启用中的对照里查找"
                };
                AppError::business_displayable(format!(
                    "款号 {product_code} 的染厂色号 {external_color_no} 没有对照到本系统的内部色号{scope}，请到供应商 SKU 对照表补上这一档对照再扫"
                ))
            }
            ResolveRejection::MappingWithoutInternalColor {
                product_code,
                external_color_no,
            } => AppError::business_displayable(format!(
                "款号 {product_code} 的染厂色号 {external_color_no} 有对照记录，但那几条对照没有指定内部色号，这一匹落不了库，请到供应商 SKU 对照表把内部色号补齐"
            )),
            ResolveRejection::AmbiguousColorMapping {
                product_code,
                external_color_no,
                candidate_count,
            } => AppError::business_displayable(format!(
                "款号 {product_code} 的染厂色号 {external_color_no} 对照到了 {candidate_count} 个不同的内部色号，系统不会取第一条入库，请把 SKU 对照收敛成一个内部色号"
            )),
            ResolveRejection::InternalColorRowMissing => AppError::business_displayable(
                "这一档对照指向的内部色号在产品色号档案里已经不存在，对照已失效，请先修好对照再扫",
            ),
            ResolveRejection::InternalColorProductMismatch => AppError::business_displayable(
                "这一档对照把染厂色号对照到了别的产品下的内部色号，跨产品的对照不能用于入库，请先修好对照再扫",
            ),
            ResolveRejection::PositionNotFound { position_no } => {
                AppError::business_displayable(format!(
                    "扫到的库位 {position_no} 在库位档案里没有登记，请先在对应仓库登记这个库位，或核对库位牌上的编码与系统是否一致"
                ))
            }
            ResolveRejection::PositionAmbiguous { position_no } => {
                AppError::business_displayable(format!(
                    "扫到的库位 {position_no} 在不止一个仓库里登记了同一个编码，系统无法判定入哪一个仓，请把库位编码改成按仓库区分后再扫"
                ))
            }
            ResolveRejection::AlreadyInbound {
                duplicate_count,
                warehouse_code,
                warehouse_in_at,
            } => {
                if duplicate_count > 1 {
                    return AppError::business_displayable(
                        "同款号同缸号同匹号的染色匹在库里已有多行重复记录，系统不会任选一行来处理，本次不入库，请让主数据维护人先查清这些重复匹行",
                    );
                }
                let Some(warehouse_code) = warehouse_code else {
                    return AppError::business_displayable(
                        "这一匹已经入过库，但它原先所在仓库的编码已经查不到了，本次不入库，请让主数据维护人核查这一匹的仓库归属",
                    );
                };
                let when = match warehouse_in_at {
                    Some(at) => at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    None => "未记录，这一匹行是入库时间列为空的存量数据".to_string(),
                };
                AppError::business_displayable(format!(
                    "这一匹已经入过库，先前入到仓库 {warehouse_code}，入库时间 {when}。重复扫码不会再生成第二条匹记录，要换位置请走移库或库存调整"
                ))
            }
            ResolveRejection::PositionMismatchOnExisting {
                existing_position_no,
                scanned_position_no,
                warehouse_code,
                warehouse_in_at,
            } => {
                let existing = match existing_position_no {
                    Some(no) => no,
                    None => "未记录库位".to_string(),
                };
                let prior = match (&warehouse_code, warehouse_in_at) {
                    (Some(code), Some(at)) => format!(
                        "先前入到仓库 {code}，入库时间 {}",
                        at.format("%Y-%m-%d %H:%M:%S")
                    ),
                    (Some(code), None) => {
                        format!(
                            "先前入到仓库 {code}，入库时间未记录，这一匹行是入库时间列为空的存量数据"
                        )
                    }
                    (None, _) => "先前入库所在仓库的编码已经查不到了".to_string(),
                };
                AppError::business_displayable(format!(
                    "这一匹已经入库，{prior}，它当前所在库位是 {existing}，与本次扫到的库位 {scanned_position_no} 不一致。要挪位置请走移库，不要按重复入库再扫一次"
                ))
            }
        }
    }
}

/// 把二维码定长切分的拒绝原因翻成 AppError。
///
/// 做什么，逐条给出不合形的原因与下一步动作。文案不点名二维码原文，因为切分失败时
/// 串里哪一段可信都未知，原文由调用点写进日志供排查。
/// 调用方，register_scanned_roll 的切分失败分支。
/// 入参，rej，一个 QrRejection 变体。
/// 出参消费方，axum 的 IntoResponse 经 AppError 出统一错误信封。
/// 存什么，不存任何东西。
/// 存哪里，无落库。
fn qr_rejection_error(rej: QrRejection) -> AppError {
    let text = match rej {
        QrRejection::WhitespaceOrControl => {
            "二维码里读到了空格或不可见字符，扫码枪常把回车一起读进来，标签污损也会读进空白。系统不裁剪空白，因为被裁掉的字符很可能就是缸号的一部分，请重新扫一次完整的码串"
        }
        QrRejection::TooShort => {
            "二维码长度不够，连款号、染厂色号与匹号三段都放不下，请确认扫到的是整张布卷标签上的码串，不是子码或残缺码"
        }
        QrRejection::EmptyDyeLot => {
            "二维码的长度只够款号、染厂色号与匹号三段，缸号段一个字符都没有，请核对标签上的缸号是否漏印或漏扫"
        }
        QrRejection::ProductCodeNotNumeric => {
            "二维码前六位款号里出现了数字以外的字符，本系统要求的款号是六位补零的纯数字，请对照标签核对款号，或确认扫的是本系统的布卷码"
        }
        QrRejection::ExternalColorNotNumeric => {
            "二维码第七到第十三位的染厂色号里出现了数字以外的字符，本系统要求的染厂色号是七位纯数字，请对照标签核对色号"
        }
        QrRejection::DyeLotNotAlphanumeric => {
            "二维码的缸号段里出现了字母与数字以外的字符，比如连字符或小数点，请对照标签重扫，缸号必须是一段连续的字母加数字"
        }
        QrRejection::PieceNoNotNumeric => {
            "二维码最后三位不是三位数字匹号，说明这一串不是本系统的布卷码，或者匹号部分被污损读错了，请对照标签核对末三位"
        }
    };
    AppError::validation_displayable(text)
}

/// 扫到的库位编码到库位身份的定位。
///
/// 做什么，按库位编码查库位档案，命中唯一一行时交付库位 ID 与它所属的仓库 ID。
/// 调用方，resolve_roll_identity。
/// 入参，db 只读连接，position_no 操作工先扫到的库位编码原文。
/// 出参消费方，resolve_roll_identity 把它装进 ResolvedRollIdentity，登记腿据此写匹行的
/// 库位列与仓库列。
/// 存什么，什么都不存，本函数只读。
/// 存哪里，无落库。
async fn resolve_position<C: ConnectionTrait>(
    db: &C,
    position_no: &str,
) -> Result<(i32, i32), ResolveFailure> {
    // 只投影需要的两列，整行读取会因历史行的可空列与模型声明漂移而解码失败
    let hits: Vec<(i32, i32)> = location::Entity::find()
        .filter(location::Column::LocationCode.eq(position_no))
        .select_only()
        .column(location::Column::Id)
        .column(location::Column::WarehouseId)
        .into_tuple::<(i32, i32)>()
        .all(db)
        .await?;

    // 库位编码的唯一索引是仓库 ID 与库位编码的两列组合，同一个编码跨仓重复是允许的，
    // 命中多行时必须拒绝而不是挑一行，否则会把布入到另一个仓库名下。
    match hits.as_slice() {
        [] => Err(ResolveRejection::PositionNotFound {
            position_no: position_no.to_string(),
        }
        .into()),
        [(location_id, warehouse_id)] => Ok((*location_id, *warehouse_id)),
        _ => Err(ResolveRejection::PositionAmbiguous {
            position_no: position_no.to_string(),
        }
        .into()),
    }
}

/// 二维码里的六位款号到我方产品 ID 的定位。
///
/// 做什么，按 products.code 等值查款号，命中一条才交付产品 ID，零条与多条各自拒绝。
/// 调用方，resolve_roll_identity。
/// 入参，db 只读连接，product_code 切分得出的六位款号原文。
/// 出参消费方，resolve_roll_identity 用它查内部色号与查重并写进匹行的产品列。
/// 存什么，什么都不存，本函数只读。
/// 存哪里，无落库，多条命中的条数只由调用点写日志。
async fn resolve_product_id<C: ConnectionTrait>(
    db: &C,
    product_code: &str,
) -> Result<i32, ResolveFailure> {
    // 只投影主键，整行读取会撞 products 表历史稀疏行的列型漂移
    let product_ids: Vec<i32> = product::Entity::find()
        .filter(product::Column::Code.eq(product_code))
        .select_only()
        .column(product::Column::Id)
        .into_tuple::<i32>()
        .all(db)
        .await?;

    match product_ids.as_slice() {
        [] => Err(ResolveRejection::ProductNotFound {
            product_code: product_code.to_string(),
        }
        .into()),
        [one] => Ok(*one),
        many => {
            tracing::warn!(
                product_code = %product_code,
                hit_count = many.len(),
                "扫码入库款号命中多条产品记录"
            );
            Err(ResolveRejection::ProductAmbiguous {
                product_code: product_code.to_string(),
            }
            .into())
        }
    }
}

/// 染厂侧外部色号到内部色号的定位。
///
/// 做什么，经供应商 SKU 对照表把二维码里的染厂色号换成内部色号 ID 与内部色号编码，
/// 给出 supplier_id 时把查找限定在该供应商的启用对照内，否则扫该款全部启用对照。
/// 调用方，resolve_roll_identity。
/// 入参，db 只读连接，product_id 已定位的我方产品 ID，parsed 切分得出的四段，
/// supplier_id 可选的供应商范围。
/// 出参消费方，resolve_roll_identity 把内部色号 ID 与编码装进 ResolvedRollIdentity，
/// 编码最终写进匹行的色号列。
/// 存什么，什么都不存，本函数只读。
/// 存哪里，无落库，多候选的具体候选 ID 只在本函数记日志。
async fn resolve_internal_color<C: ConnectionTrait>(
    db: &C,
    product_id: i32,
    parsed: &ParsedRollQr,
    supplier_id: Option<i32>,
) -> Result<(i32, String), ResolveFailure> {
    // 染厂色号是外部编码，只有供应商色号行携带它，所以对照表到供应商色号行走内连。
    // 只取对照行的内部色号列，候选收敛成去重列表后再判条数，绝不排序取第一条。
    let mut mapping_query = product_supplier_mapping::Entity::find()
        .filter(product_supplier_mapping::Column::ProductId.eq(product_id))
        .filter(product_supplier_mapping::Column::IsEnabled.eq(true))
        .join(
            JoinType::InnerJoin,
            product_supplier_mapping::Relation::SupplierProductColor.def(),
        )
        .filter(supplier_product_color::Column::ColorNo.eq(&parsed.external_color_no));
    if let Some(supplier_id) = supplier_id {
        mapping_query =
            mapping_query.filter(product_supplier_mapping::Column::SupplierId.eq(supplier_id));
    }
    let mapped_color_ids: Vec<Option<i32>> = mapping_query
        .select_only()
        .column(product_supplier_mapping::Column::ProductColorId)
        .into_tuple::<Option<i32>>()
        .all(db)
        .await?;

    if mapped_color_ids.is_empty() {
        return Err(ResolveRejection::NoColorMapping {
            product_code: parsed.product_code.clone(),
            external_color_no: parsed.external_color_no.clone(),
            supplier_scoped: supplier_id.is_some(),
        }
        .into());
    }

    // 去重保留首次出现顺序，同一内部色号被多条对照重复命中只算一个候选
    let mut distinct_color_ids: Vec<i32> = Vec::new();
    for color_id in mapped_color_ids.iter().copied().flatten() {
        if !distinct_color_ids.contains(&color_id) {
            distinct_color_ids.push(color_id);
        }
    }
    if distinct_color_ids.is_empty() {
        return Err(ResolveRejection::MappingWithoutInternalColor {
            product_code: parsed.product_code.clone(),
            external_color_no: parsed.external_color_no.clone(),
        }
        .into());
    }
    if distinct_color_ids.len() > 1 {
        tracing::warn!(
            product_code = %parsed.product_code,
            external_color_no = %parsed.external_color_no,
            candidate_ids = ?distinct_color_ids,
            "扫码入库染厂色号对照到多个内部色号，拒绝任选一条"
        );
        return Err(ResolveRejection::AmbiguousColorMapping {
            product_code: parsed.product_code.clone(),
            external_color_no: parsed.external_color_no.clone(),
            candidate_count: distinct_color_ids.len(),
        }
        .into());
    }
    let product_color_id = distinct_color_ids[0];

    // 内部色号行必须真实存在且归属刚定位到的产品，对照漂移时宁可拒也不落错色
    let color_row: Option<(String, i32)> = product_color::Entity::find()
        .filter(product_color::Column::Id.eq(product_color_id))
        .select_only()
        .column(product_color::Column::ColorNo)
        .column(product_color::Column::ProductId)
        .into_tuple::<(String, i32)>()
        .one(db)
        .await?;
    match color_row {
        None => {
            tracing::warn!(
                product_color_id,
                product_code = %parsed.product_code,
                "扫码入库对照指向的内部色号行不存在"
            );
            Err(ResolveRejection::InternalColorRowMissing.into())
        }
        Some((_, owner_product_id)) if owner_product_id != product_id => {
            tracing::warn!(
                product_color_id,
                owner_product_id,
                product_id,
                "扫码入库对照指向的内部色号归属另一个产品"
            );
            Err(ResolveRejection::InternalColorProductMismatch.into())
        }
        Some((internal_color_no, _)) => Ok((product_color_id, internal_color_no)),
    }
}

/// 同款号同缸号同匹号的重复入库防护。
///
/// 做什么，按款号与缸号与匹号查已存在的染色匹，命中就带着先前那一匹的仓库编码与入库时间
/// 一起拒绝，让操作工看到先前那次入库而不是静默落下第二行。先前记录的库位与本次扫到的
/// 库位不同时，改判为挪位诉求并点名该走移库。
/// 调用方，resolve_roll_identity。
/// 入参，db 只读连接，product_id 已定位的我方产品 ID，parsed 切分得出的四段，
/// scanned_position_no 本次扫到的库位编码原文。
/// 出参消费方，调用方只消费 Ok 这个判据本身，拒绝材料经 ResolveRejection 直达扫码入库端点
/// 的错误响应，命中条数与仓库归属由本函数记日志。
/// 存什么，什么都不存，本函数只读。
/// 存哪里，无落库。
async fn assert_not_already_inbound<C: ConnectionTrait>(
    db: &C,
    product_id: i32,
    parsed: &ParsedRollQr,
    scanned_position_no: &str,
) -> Result<(), ResolveFailure> {
    // 查重口径与出库四维 tuple 同源，即款号加缸号加匹号再加匹类型，
    // 不按匹号裸匹配，因为匹号跨缸可重复。
    // 本腿写入的匹行 dye_lot_id 为空，m0051 的部分唯一索引只覆盖（dye_lot_id、piece_no）
    // 组合，够不到这一形态，所以重复防护只能落在这一层。
    let prior: Vec<(i32, Option<String>, Option<chrono::DateTime<chrono::Utc>>)> =
        inventory_piece::Entity::find()
            .filter(inventory_piece::Column::ProductId.eq(product_id))
            .filter(inventory_piece::Column::DyeLotNo.eq(&parsed.dye_lot_no))
            .filter(inventory_piece::Column::PieceNo.eq(&parsed.piece_no))
            .filter(inventory_piece::Column::PieceType.eq(PIECE_TYPE_DYED))
            .select_only()
            .column(inventory_piece::Column::WarehouseId)
            .column(inventory_piece::Column::PositionNo)
            .column(inventory_piece::Column::WarehouseInAt)
            .into_tuple::<(i32, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>()
            .all(db)
            .await?;
    if prior.is_empty() {
        return Ok(());
    }

    tracing::warn!(
        product_id,
        product_code = %parsed.product_code,
        dye_lot_no = %parsed.dye_lot_no,
        piece_no = %parsed.piece_no,
        duplicate_count = prior.len(),
        prior_warehouse_id = ?prior.first().map(|row| row.0),
        "扫码入库查重命中，该匹已在库，本次不再生成匹记录"
    );

    // 一行命中才把先前入库的仓库编码与入库时间带给操作工。
    // 多行属数据异常，此时任选一行的仓库来报就是在编造事实，只报重复这件事。
    if prior.len() > 1 {
        return Err(ResolveRejection::AlreadyInbound {
            duplicate_count: prior.len(),
            warehouse_code: None,
            warehouse_in_at: None,
        }
        .into());
    }
    let (prior_warehouse_id, prior_position_no, prior_in_at) = &prior[0];
    let warehouse_code: Option<String> = warehouse::Entity::find()
        .filter(warehouse::Column::Id.eq(*prior_warehouse_id))
        .select_only()
        .column(warehouse::Column::WarehouseCode)
        .into_tuple::<String>()
        .one(db)
        .await?;
    if warehouse_code.is_none() {
        tracing::warn!(
            prior_warehouse_id,
            piece_no = %parsed.piece_no,
            "扫码入库查重命中行的仓库编码取不出来，该匹行的仓库归属已断链"
        );
    }
    // 库位不一致时先点名挪位面，重复事实与先前入库的仓库和入库时间一并带回。
    if prior_position_no.as_deref() != Some(scanned_position_no) {
        return Err(ResolveRejection::PositionMismatchOnExisting {
            existing_position_no: prior_position_no.clone(),
            scanned_position_no: scanned_position_no.to_string(),
            warehouse_code,
            warehouse_in_at: *prior_in_at,
        }
        .into());
    }
    Err(ResolveRejection::AlreadyInbound {
        duplicate_count: 1,
        warehouse_code,
        warehouse_in_at: *prior_in_at,
    }
    .into())
}

/// 二维码四段加库位编码到内部身份的定位与查重。
///
/// 做什么，款号换产品 ID，库位编码换库位与仓库，染厂色号换内部色号，
/// 最后查该染色匹是否已入库。
/// 调用方，register_scanned_roll。
/// 入参，db 只读连接，parsed 定长切分的四段，supplier_id 可选的供应商范围，
/// position_no 先扫到的库位编码。
/// 出参消费方，register_scanned_roll 用交付的内部身份写库存匹行。
/// 存什么，什么都不存，本函数只读。
/// 存哪里，无落库。
async fn resolve_roll_identity<C: ConnectionTrait>(
    db: &C,
    parsed: &ParsedRollQr,
    supplier_id: Option<i32>,
    position_no: &str,
) -> Result<ResolvedRollIdentity, ResolveFailure> {
    let product_id = resolve_product_id(db, &parsed.product_code).await?;
    // 库位先定位，操作工先扫到的是库位，库位不合形时报库位比报色号更贴近他刚做的事
    let (location_id, warehouse_id) = resolve_position(db, position_no).await?;
    let (product_color_id, internal_color_no) =
        resolve_internal_color(db, product_id, parsed, supplier_id).await?;
    assert_not_already_inbound(db, product_id, parsed, position_no).await?;

    Ok(ResolvedRollIdentity {
        product_id,
        product_color_id,
        internal_color_no,
        location_id,
        warehouse_id,
    })
}

/// 扫码入库的一次完整登记，切分、定位、查重、落一行库存匹。
///
/// 做什么，把二维码原文按定长切分，按款号与染厂色号与库位定位内部身份，
/// 查重通过后写且只写一行库存匹，匹类型为染色匹，条码列写二维码原文。
/// 调用方，handlers 下的 barcode_scanner_handler 的扫码入库端点。
/// 入参，db 数据库连接，cmd 本次扫码的请求值，含二维码原文、库位编码、可选供应商范围、
/// 可选长度、可选重量与操作人 ID，长度缺值与零与负数各自显式拒绝。
/// 出参消费方，扫码入库端点把返回的匹行序列化后回给移动端，移动端展示这一匹的入库结果。
/// 存什么，存一行库存匹，含匹号、匹类型、缸号、款号、内部色号、库位、仓库、长度、重量、
/// 入库时间、创建人与条码原文，批号列按本域既有口径写缸号。
/// 存哪里，库存匹表 inventory_piece 的一行，由本函数经 SeaORM 的 insert 写入。
pub async fn register_scanned_roll<C: ConnectionTrait>(
    db: &C,
    cmd: &ScanInboundCommand<'_>,
) -> Result<inventory_piece::Model, AppError> {
    if cmd.position_no.is_empty() {
        return Err(AppError::validation_displayable(
            "还没有扫到库位，请先扫库位码再扫布卷码，入库必须落到具体库位",
        ));
    }

    let parsed = parse_inbound_qr(cmd.raw_qr).map_err(|rej| {
        tracing::warn!(
            raw_qr_len = cmd.raw_qr.chars().count(),
            reason = ?rej,
            "扫码入库二维码定长切分被拒"
        );
        qr_rejection_error(rej)
    })?;

    // 长度必填且必须为正，缺值与零与负数各自显式拒绝。
    // 为什么不在缺值时兜底成 0，匹行的米数是下游所有账面的计量源，
    // 零米数的匹行会让这一匹在库存、出库与对账里全部算错，而且看起来是已实测的。
    let Some(length) = cmd.length else {
        return Err(AppError::validation_displayable(
            "这一匹还没有填写长度，扫码入库必须带实测米数，请填写这一匹的长度后重新提交",
        ));
    };
    if length <= rust_decimal::Decimal::ZERO {
        return Err(AppError::validation_displayable(
            "这一匹的长度必须大于 0 米，请填写实测米数后再提交，填 0 或负数都会被拒绝",
        ));
    }
    // 重量复用匹域既有的取值域门，留空不判，有值必须是正值
    validate_piece_measured_value(
        "扫码入库",
        "重量(weight)",
        Some(parsed.piece_no.as_str()),
        cmd.weight,
    )?;

    let identity = resolve_roll_identity(db, &parsed, cmd.supplier_id, cmd.position_no).await?;

    // 仓库类型与匹类型的兼容门，染色匹只能入成品仓，与生产报工和委外收回两个写入点同口径
    validate_warehouse_for_piece_type(db, identity.warehouse_id, PIECE_TYPE_DYED, false).await?;

    // 与 create_greige_pieces_from_report 同一时钟口径
    let now_utc = crate::utils::date_utils::utc_now_fixed().with_timezone(&chrono::Utc);
    let active = inventory_piece::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        piece_no: Set(parsed.piece_no.clone()),
        piece_type: Set(PIECE_TYPE_DYED.to_string()),
        machine_no: Set(None),
        machine_operator: Set(None),
        warehouse_in_at: Set(Some(now_utc)),
        // 本腿不建缸号档案，匹行只落缸号字符串，所以 dye_lot_id 留空。
        // 缸号档案的产生时机在染色与委外收回两侧，扫码入库接收的是外部已染好的布，
        // 在这里凭空补一条档案会造出没有对应染色事实的缸号。
        dye_lot_id: Set(None),
        dye_lot_no: Set(parsed.dye_lot_no.clone()),
        color_no: Set(identity.internal_color_no.clone()),
        // 染色匹的批号口径与委外收回写入点一致，批号即缸号
        batch_no: Set(parsed.dye_lot_no.clone()),
        product_id: Set(identity.product_id),
        warehouse_id: Set(identity.warehouse_id),
        location_id: Set(Some(identity.location_id)),
        position_no: Set(Some(cmd.position_no.to_string())),
        length: Set(length),
        // 重量按请求原样落，缺值保持 NULL，由标签侧 fail-closed 点名，此处不兜底
        weight: Set(cmd.weight),
        width: Set(None),
        gram_weight: Set(None),
        production_date: Set(None),
        shelf_life: Set(None),
        quality_status: Set(None),
        inventory_status: Set(Some(piece_status::AVAILABLE.to_string())),
        supplier_piece_no: Set(None),
        package_no: Set(None),
        barcode: Set(Some(cmd.raw_qr.to_string())),
        parent_piece_id: Set(None),
        inspection_id: Set(None),
        // 匹号来自二维码本身，不是系统内的缸号序号，所以不写 piece_seq
        piece_seq: Set(None),
        // 扫码类型词表目前只有 SHIP 与 INVENTORY 两值，入库值尚未定案，此处不新造词
        scan_type: Set(None),
        status: Set(piece_status::AVAILABLE.to_string()),
        remarks: Set(Some(format!("扫码入库逐匹登记，库位 {}", cmd.position_no))),
        created_at: Set(now_utc),
        updated_at: Set(now_utc),
        created_by: Set(Some(cmd.operator_id)),
        updated_by: Set(None),
        original_length: Set(None),
        original_weight: Set(None),
    };
    let created = active.insert(db).await?;

    tracing::info!(
        piece_id = created.id,
        piece_no = %created.piece_no,
        dye_lot_no = %created.dye_lot_no,
        warehouse_id = created.warehouse_id,
        "扫码入库登记完成"
    );
    Ok(created)
}
