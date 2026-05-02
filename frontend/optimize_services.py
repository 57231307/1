import re
import os

# 1. purchase_receipt_service.rs
with open("src/services/purchase_receipt_service.rs", "r") as f:
    content = f.read()

replacement = """    /// 获取收货单列表
    pub async fn list(query: PurchaseReceiptQuery) -> Result<PaginatedReceipts, String> {
        <Self as CrudService>::list_with_query(&query).await
    }

    /// 获取收货单详情
    pub async fn get(id: i32) -> Result<PurchaseReceipt, String> {
        <Self as CrudService>::get(id).await
    }

    /// 创建收货单
    pub async fn create(req: CreatePurchaseReceiptRequest) -> Result<PurchaseReceipt, String> {
        <Self as CrudService>::create(req).await
    }

    /// 更新收货单
    pub async fn update(id: i32, req: UpdatePurchaseReceiptRequest) -> Result<PurchaseReceipt, String> {
        <Self as CrudService>::update(id, req).await
    }

    /// 删除收货单
    pub async fn delete(id: i32) -> Result<(), String> {
        <Self as CrudService>::delete(id).await
    }"""

# Remove old list/get/create/update/delete implementation
# From `pub async fn list` to before `pub async fn confirm`
start_idx = content.find("    /// 获取收货单列表")
end_idx = content.find("    /// 确认收货单")
if start_idx != -1 and end_idx != -1:
    content = content[:start_idx] + replacement + "\n\n" + content[end_idx:]

with open("src/services/purchase_receipt_service.rs", "w") as f:
    f.write(content)


# 2. budget_management_service.rs
with open("src/services/budget_management_service.rs", "r") as f:
    content = f.read()

replacement2 = """    /// 获取预算科目列表
    pub async fn list_items(query: BudgetItemQuery) -> Result<BudgetItemListResponse, String> {
        <Self as CrudService>::list_with_query(&query).await
    }

    /// 获取预算科目详情
    pub async fn get_item(id: i32) -> Result<BudgetItem, String> {
        <Self as CrudService>::get(id).await
    }

    /// 创建预算科目
    pub async fn create_item(req: CreateBudgetItemRequest) -> Result<BudgetItem, String> {
        <Self as CrudService>::create(req).await
    }

    /// 更新预算科目
    pub async fn update_item(id: i32, req: UpdateBudgetItemRequest) -> Result<BudgetItem, String> {
        <Self as CrudService>::update(id, req).await
    }

    /// 删除预算科目
    pub async fn delete_item(id: i32) -> Result<(), String> {
        <Self as CrudService>::delete(id).await
    }"""

start_idx2 = content.find("    /// 获取预算科目列表")
end_idx2 = content.find("    /// 获取预算方案列表")
if start_idx2 != -1 and end_idx2 != -1:
    content = content[:start_idx2] + replacement2 + "\n\n" + content[end_idx2:]

with open("src/services/budget_management_service.rs", "w") as f:
    f.write(content)

