use chrono::{Utc, NaiveDate};
// �ļ����ι�������
// �ṩ��Ʒ-ɫ��-�׺�-ƥ�ŵ�����������??
use std::sync::Arc;
use rust_decimal::Decimal;
use sea_orm::{Set, PaginatorTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, ColumnTrait, ActiveModelTrait, DbErr, QuerySelect};;
use crate::models::{
    batch_dye_lot, inventory_piece, product_code_mapping, color_code_mapping,
    dye_lot_mapping, piece_mapping, batch_trace_log, product, product_color
};
use serde::{Deserialize, Serialize};

// ����ģ��
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateDyeLotRequest {
    pub product_id: i32,
    pub color_id: i32,
    pub supplier_dye_lot_no: String,
    pub supplier_id: i32,
    pub production_date: Option<NaiveDate>,
    pub machine_no: Option<String>,
    pub batch_weight: Option<Decimal>,
    pub total_length: Option<Decimal>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
    pub created_by: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePieceRequest {
    pub dye_lot_id: i32,
    pub supplier_piece_no: String,
    pub length: Decimal,
    pub weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    pub position_no: Option<String>,
    pub package_no: Option<String>,
    pub production_date: Option<NaiveDate>,
    pub shelf_life: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MapCodeRequest {
    pub internal_code: String,
    pub supplier_code: String,
    pub supplier_id: i32,
    pub code_type: String, // "product", "color", "dye_lot", "piece"
    pub mapped_by: i32,
}

// ���ģ��
#[derive(Debug, Serialize)]
pub struct DyeLotInfo {
    pub id: i32,
    pub dye_lot_no: String,
    pub product_id: i32,
    pub color_id: i32,
    pub supplier_dye_lot_no: String,
    pub supplier_id: i32,
    pub production_date: Option<NaiveDate>,
    pub machine_no: Option<String>,
    pub batch_weight: Option<Decimal>,
    pub total_length: Option<Decimal>,
    pub total_pieces: i32,
    pub quality_grade: String,
    pub quality_status: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct PieceInfo {
    pub id: i32,
    pub piece_no: String,
    pub dye_lot_id: i32,
    pub supplier_piece_no: String,
    pub length: Decimal,
    pub weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    pub position_no: Option<String>,
    pub package_no: Option<String>,
    pub quality_status: String,
    pub inventory_status: String,
    pub warehouse_id: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct FourLevelBatchService {
    pub db: Arc<DatabaseConnection>,
}

impl FourLevelBatchService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// �����׺ż�¼
    pub async fn create_dye_lot(&self, req: CreateDyeLotRequest) -> Result<DyeLotInfo, DbErr> {
        let db = &*self.db;
        
        // ���ɸ׺ű��
        let dye_lot_no = format!("DL{}{:06}", 
            Utc::now().format("%Y%m%d"), 
            self.generate_sequence_number().await?
        );

        let dye_lot = batch_dye_lot::ActiveModel {
            dye_lot_no: Set(dye_lot_no.clone()),
            product_id: Set(req.product_id),
            color_id: Set(req.color_id),
            supplier_dye_lot_no: Set(Some(req.supplier_dye_lot_no)),
            supplier_id: Set(req.supplier_id),
            production_date: Set(req.production_date),
            machine_no: Set(req.machine_no),
            batch_weight: Set(req.batch_weight),
            total_length: Set(req.total_length),
            quality_grade: Set(req.quality_grade.unwrap_or_else(|| "A".to_string())),
            quality_status: Set("pending".to_string()),
            remarks: Set(req.remarks),
            is_active: Set(true),
            created_by: Set(Some(req.created_by)),
            updated_by: Set(Some(req.created_by)),
            ..Default::default()
        };

        let inserted_dye_lot = dye_lot.insert(db).await?;
        
        Ok(DyeLotInfo {
            id: inserted_dye_lot.id,
            dye_lot_no: inserted_dye_lot.dye_lot_no,
            product_id: inserted_dye_lot.product_id,
            color_id: inserted_dye_lot.color_id,
            supplier_dye_lot_no: inserted_dye_lot.supplier_dye_lot_no.unwrap_or_default(),
            supplier_id: inserted_dye_lot.supplier_id,
            production_date: inserted_dye_lot.production_date,
            machine_no: inserted_dye_lot.machine_no,
            batch_weight: inserted_dye_lot.batch_weight,
            total_length: inserted_dye_lot.total_length,
            total_pieces: inserted_dye_lot.total_pieces,
            quality_grade: inserted_dye_lot.quality_grade,
            quality_status: inserted_dye_lot.quality_status,
            is_active: inserted_dye_lot.is_active,
            created_at: inserted_dye_lot.created_at.into(),
        })
    }

    /// ����ƥ�ż�¼
    pub async fn create_piece(&self, req: CreatePieceRequest) -> Result<PieceInfo, DbErr> {
        let db = &*self.db;

        // ����ƥ�ű��
        let piece_no = format!("P{}{:08}", 
            Utc::now().format("%Y%m%d"), 
            self.generate_sequence_number().await?
        );

        let piece = inventory_piece::ActiveModel {
            piece_no: Set(piece_no.clone()),
            dye_lot_id: Set(req.dye_lot_id),
            supplier_piece_no: Set(Some(req.supplier_piece_no)),
            length: Set(req.length),
            weight: Set(req.weight),
            width: Set(req.width),
            gram_weight: Set(req.gram_weight),
            position_no: Set(req.position_no),
            package_no: Set(req.package_no),
            production_date: Set(req.production_date),
            shelf_life: Set(req.shelf_life),
            quality_status: Set("pending".to_string()),
            inventory_status: Set("available".to_string()),
            warehouse_id: Set(req.warehouse_id),
            remarks: Set(req.remarks),
            created_by: Set(Some(req.created_by)),
            updated_by: Set(Some(req.created_by)),
            ..Default::default()
        };

        let inserted_piece = piece.insert(db).await?;
        
        Ok(PieceInfo {
            id: inserted_piece.id,
            piece_no: inserted_piece.piece_no,
            dye_lot_id: inserted_piece.dye_lot_id,
            supplier_piece_no: inserted_piece.supplier_piece_no.unwrap_or_default(),
            length: inserted_piece.length,
            weight: inserted_piece.weight,
            width: inserted_piece.width,
            gram_weight: inserted_piece.gram_weight,
            position_no: inserted_piece.position_no,
            package_no: inserted_piece.package_no,
            quality_status: inserted_piece.quality_status,
            inventory_status: inserted_piece.inventory_status,
            warehouse_id: inserted_piece.warehouse_id,
            created_at: inserted_piece.created_at.into(),
        })
    }

    /// ��ȡ�׺�����
    pub async fn get_dye_lot_by_id(&self, id: i32) -> Result<Option<DyeLotInfo>, DbErr> {
        let db = &*self.db;
        
        let dye_lot = batch_dye_lot::Entity::find_by_id(id)
            .one(db)
            .await?;
            
        match dye_lot {
            Some(dye_lot) => Ok(Some(DyeLotInfo {
                id: dye_lot.id,
                dye_lot_no: dye_lot.dye_lot_no,
                product_id: dye_lot.product_id,
                color_id: dye_lot.color_id,
                supplier_dye_lot_no: dye_lot.supplier_dye_lot_no.unwrap_or_default(),
                supplier_id: dye_lot.supplier_id,
                production_date: dye_lot.production_date,
                machine_no: dye_lot.machine_no,
                batch_weight: dye_lot.batch_weight,
                total_length: dye_lot.total_length,
                total_pieces: dye_lot.total_pieces,
                quality_grade: dye_lot.quality_grade,
                quality_status: dye_lot.quality_status,
                is_active: dye_lot.is_active,
                created_at: dye_lot.created_at.into(),
            })),
            None => Ok(None),
        }
    }

    /// ��ȡ�׺��µ�����ƥ??    pub async fn get_pieces_by_dye_lot(&self, dye_lot_id: i32) -> Result<Vec<PieceInfo>, DbErr> {
        let db = &*self.db;
        
        let pieces = inventory_piece::Entity::find()
            .filter(inventory_piece::Column::DyeLotId.eq(dye_lot_id))
            .order_by(inventory_piece::Column::Id, Order::Asc)
            .all(db)
            .await?;
            
        Ok(pieces.into_iter().map(|piece| PieceInfo {
            id: piece.id,
            piece_no: piece.piece_no,
            dye_lot_id: piece.dye_lot_id,
            supplier_piece_no: piece.supplier_piece_no.unwrap_or_default(),
            length: piece.length,
            weight: piece.weight,
            width: piece.width,
            gram_weight: piece.gram_weight,
            position_no: piece.position_no,
            package_no: piece.package_no,
            quality_status: piece.quality_status,
            inventory_status: piece.inventory_status,
            warehouse_id: piece.warehouse_id,
            created_at: piece.created_at.into(),
        }).collect())
    }

    /// ��������ӳ��
    pub async fn create_code_mapping(&self, req: MapCodeRequest) -> Result<bool, DbErr> {
        let db = &*self.db;
        
        let mapping_date = Utc::now().date_naive();
        
        match req.code_type.as_str() {
            "product" => {
                let mapping = product_code_mapping::ActiveModel {
                    internal_product_code: Set(req.internal_code),
                    supplier_product_code: Set(req.supplier_code),
                    supplier_id: Set(req.supplier_id),
                    is_active: Set(true),
                    mapping_date: Set(mapping_date),
                    validation_status: Set("pending".to_string()),
                    created_by: Set(Some(req.mapped_by)),
                    updated_by: Set(Some(req.mapped_by)),
                    ..Default::default()
                };
                mapping.insert(db).await?;
            },
            "color" => {
                let mapping = color_code_mapping::ActiveModel {
                    internal_color_no: Set(req.internal_code),
                    supplier_color_code: Set(req.supplier_code),
                    supplier_id: Set(req.supplier_id),
                    is_active: Set(true),
                    mapping_date: Set(mapping_date),
                    validation_status: Set("pending".to_string()),
                    created_by: Set(Some(req.mapped_by)),
                    updated_by: Set(Some(req.mapped_by)),
                    ..Default::default()
                };
                mapping.insert(db).await?;
            },
            _ => return Err(DbErr::Custom(format!("Unsupported code type: {}", req.code_type))),
        }
        
        Ok(true)
    }

    /// �����ڲ������ȡ��Ӧ�̱�??    pub async fn get_supplier_code_by_internal(&self, internal_code: &str, code_type: &str, supplier_id: i32) -> Result<Option<String>, DbErr> {
        let db = &*self.db;
        
        match code_type {
            "product" => {
                let mapping = product_code_mapping::Entity::find()
                    .filter(product_code_mapping::Column::InternalProductCode.eq(internal_code))
                    .filter(product_code_mapping::Column::SupplierId.eq(supplier_id))
                    .filter(product_code_mapping::Column::IsActive.eq(true))
                    .one(db)
                    .await?;
                    
                Ok(mapping.map(|m| m.supplier_product_code))
            },
            "color" => {
                let mapping = color_code_mapping::Entity::find()
                    .filter(color_code_mapping::Column::InternalColorNo.eq(internal_code))
                    .filter(color_code_mapping::Column::SupplierId.eq(supplier_id))
                    .filter(color_code_mapping::Column::IsActive.eq(true))
                    .one(db)
                    .await?;
                    
                Ok(mapping.map(|m| m.supplier_color_code))
            },
            _ => Ok(None),
        }
    }

    /// ���ݹ�Ӧ�̱����ȡ�ڲ���??    pub async fn get_internal_code_by_supplier(&self, supplier_code: &str, code_type: &str, supplier_id: i32) -> Result<Option<String>, DbErr> {
        let db = &*self.db;
        
        match code_type {
            "product" => {
                let mapping = product_code_mapping::Entity::find()
                    .filter(product_code_mapping::Column::SupplierProductCode.eq(supplier_code))
                    .filter(product_code_mapping::Column::SupplierId.eq(supplier_id))
                    .filter(product_code_mapping::Column::IsActive.eq(true))
                    .one(db)
                    .await?;
                    
                Ok(mapping.map(|m| m.internal_product_code))
            },
            "color" => {
                let mapping = color_code_mapping::Entity::find()
                    .filter(color_code_mapping::Column::SupplierColorCode.eq(supplier_code))
                    .filter(color_code_mapping::Column::SupplierId.eq(supplier_id))
                    .filter(color_code_mapping::Column::IsActive.eq(true))
                    .one(db)
                    .await?;
                    
                Ok(mapping.map(|m| m.internal_color_no))
            },
            _ => Ok(None),
        }
    }

    /// ��������??    async fn generate_sequence_number(&self) -> Result<i32, DbErr> {
        // �������ʵ�ָ����ӵ����к������߼�
        // ��ʵ�֣���ǰʱ�������������Ϊ���к�
        Ok((Utc::now().timestamp() % 1000000) as i32)
    }
}
