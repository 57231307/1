import re

with open("backend/src/services/purchase_order_service.rs", "r") as f:
    content = f.read()

# Add FromQueryResult for DTO
dto_code = """
use sea_orm::FromQueryResult;

#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderDto {
    pub id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub order_date: NaiveDate,
    pub expected_delivery_date: Option<NaiveDate>,
    pub actual_delivery_date: Option<NaiveDate>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub purchaser_id: i32,
    pub currency: String,
    pub exchange_rate: Decimal,
    pub total_amount: Decimal,
    pub total_amount_foreign: Decimal,
    pub total_quantity: Decimal,
    pub total_quantity_alt: Decimal,
    #[serde(rename = "status")]
    pub order_status: String,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderItemDto {
    pub id: i32,
    pub order_id: i32,
    pub line_no: i32,
    pub product_id: i32,
    #[serde(rename = "material_id")]
    pub material_id: i32,
    #[serde(rename = "material_code")]
    pub material_code: String,
    #[serde(rename = "material_name")]
    pub material_name: String,
    pub quantity: Decimal,
    #[serde(rename = "quantity_ordered")]
    pub quantity_ordered: Decimal,
    pub unit_price: Decimal,
    pub tax_percent: Decimal,
    #[serde(rename = "tax_rate")]
    pub tax_rate: Decimal,
    pub amount: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
    pub received_quantity: Decimal,
    pub returned_quantity: Decimal,
    pub notes: Option<String>,
}
"""

if "PurchaseOrderDto" not in content:
    content = content.replace("use sea_orm::entity::prelude::*;", "use sea_orm::entity::prelude::*;\n" + dto_code)

# Replace list_orders signature
content = re.sub(
    r"pub async fn list_orders\(.*?\).*?Result<\(Vec<purchase_order::Model>, u64\), AppError> \{",
    r"""pub async fn list_orders(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<(Vec<PurchaseOrderDto>, u64), AppError> {""",
    content,
    flags=re.DOTALL
)

# Replace get_order signature
content = re.sub(
    r"pub async fn get_order\(&self, order_id: i32\) -> Result<purchase_order::Model, AppError> \{",
    r"""pub async fn get_order(&self, order_id: i32) -> Result<PurchaseOrderDto, AppError> {""",
    content
)

# We need to change the implementation of list_orders
list_orders_impl = """
        let mut query = purchase_order::Entity::find()
            .select_only()
            .column(purchase_order::Column::Id)
            .column(purchase_order::Column::OrderNo)
            .column(purchase_order::Column::SupplierId)
            .column_as(crate::models::supplier::Column::SupplierName, "supplier_name")
            .column(purchase_order::Column::OrderDate)
            .column(purchase_order::Column::ExpectedDeliveryDate)
            .column(purchase_order::Column::ActualDeliveryDate)
            .column(purchase_order::Column::WarehouseId)
            .column_as(crate::models::warehouse::Column::Name, "warehouse_name")
            .column(purchase_order::Column::DepartmentId)
            .column_as(crate::models::department::Column::Name, "department_name")
            .column(purchase_order::Column::PurchaserId)
            .column(purchase_order::Column::Currency)
            .column(purchase_order::Column::ExchangeRate)
            .column(purchase_order::Column::TotalAmount)
            .column(purchase_order::Column::TotalAmountForeign)
            .column(purchase_order::Column::TotalQuantity)
            .column(purchase_order::Column::TotalQuantityAlt)
            .column(purchase_order::Column::OrderStatus)
            .column(purchase_order::Column::PaymentTerms)
            .column(purchase_order::Column::ShippingTerms)
            .column(purchase_order::Column::Notes)
            .column(purchase_order::Column::CreatedAt)
            .column(purchase_order::Column::UpdatedAt)
            .left_join(crate::models::supplier::Entity)
            .left_join(crate::models::warehouse::Entity)
            .left_join(crate::models::department::Entity);

        if let Some(status) = status {
            query = query.filter(purchase_order::Column::OrderStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_order::Column::SupplierId.eq(supplier_id));
        }

        let paginator = query
            .into_model::<PurchaseOrderDto>()
            .order_by(purchase_order::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
"""

# Find the body of list_orders and replace it
# Wait, this is tricky with regex. Let's just use simple sed or rewrite.
"""
with open("backend/src/services/purchase_order_service.rs", "w") as f:
    f.write(content)
