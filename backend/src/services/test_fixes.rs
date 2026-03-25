use chrono::{Utc, NaiveDate};
// �����޸��ļ����﷨��ȷ��
// ����ļ�������֤���ǵ��޸�ģʽ�Ƿ���ȷ

use sea_orm::{
    DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter,
    ActiveModelTrait, Set, QuerySelect, QueryOrder,
    DbErr, PaginatorTrait, Order,
};

// ���� PaginatorTrait ��ʹ��
pub async fn test_paginator_usage(db: &DatabaseConnection) -> Result<(), DbErr> {
    // ����һ�����Ժ�����������֤ PaginatorTrait �ĵ����Ƿ���ȷ
    
    // ģ���ҳ��ѯģʽ
    let query = crate::models::supplier::Entity::find();
    
    let page_size = 20u64;
    let page = 0u64;
    
    let paginator = query
        .order_by(crate::models::supplier::Column::Id, Order::Desc)
        .paginate(db, page_size);
    
    let total = paginator.num_items().await?;
    let _items = paginator.fetch_page(page).await?;
    
    Ok(())
}

// ����ʱ������ת��
pub async fn test_time_conversion() {
    use chrono::Utc;
    
    let _now = Utc::now().naive_utc();
}

// ���� DatabaseConnection (���� Arc)
pub struct TestService {
    db: DatabaseConnection,
}

impl TestService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
