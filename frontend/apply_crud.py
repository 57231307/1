import os
import re

services = {
    'purchase_price_service.rs': ('PurchasePrice', 'Vec<PurchasePrice>', 'CreatePurchasePriceRequest', 'UpdatePurchasePriceRequest', '/purchases/prices'),
    'supplier_evaluation_service.rs': ('SupplierEvaluation', 'Vec<SupplierEvaluation>', 'CreateEvaluationRequest', 'UpdateEvaluationRequest', '/supplier-evaluation/evaluations'),
    'dye_batch_service.rs': ('DyeBatch', 'DyeBatchListResponse', 'CreateDyeBatchRequest', 'UpdateDyeBatchRequest', '/dye-batch'),
    'purchase_order_service.rs': ('PurchaseOrder', 'Vec<PurchaseOrder>', 'CreatePurchaseOrderRequest', 'UpdatePurchaseOrderRequest', '/purchases/orders'),
    'purchase_return_service.rs': ('PurchaseReturn', 'PaginatedReturns', 'CreatePurchaseReturnRequest', 'UpdatePurchaseReturnRequest', '/purchases/returns'),
    'ap_payment_request_service.rs': ('ApPaymentRequest', 'ApPaymentRequestListResponse', 'CreateApPaymentRequest', 'UpdateApPaymentRequest', '/ap-payment-requests'),
    'sales_analysis_service.rs': ('SalesTrendAnalysis', 'Vec<SalesTarget>', 'CreateSalesTargetRequest', 'UpdateSalesTargetRequest', '/sales-analysis/trend'),
    'greige_fabric_service.rs': ('GreigeFabric', 'GreigeFabricListResponse', 'CreateGreigeFabricRequest', 'UpdateGreigeFabricRequest', '/greige-fabric'),
    'account_subject_service.rs': ('AccountSubject', 'Vec<AccountSubject>', 'CreateSubjectRequest', 'UpdateSubjectRequest', '/account-subjects'),
    'fabric_order_service.rs': ('FabricOrder', 'Vec<FabricOrder>', 'CreateFabricOrderRequest', 'UpdateFabricOrderRequest', '/sales/fabric-orders'),
    'sales_service.rs': ('SalesOrder', 'SalesOrderListResponse', 'CreateSalesOrderRequest', 'UpdateSalesOrderRequest', '/sales/orders'),
    'ap_payment_service.rs': ('ApPayment', 'ApPaymentListResponse', 'CreateApPaymentRequest', 'UpdateApPaymentRequest', '/ap-payments'),
    'finance_invoice_service.rs': ('FinanceInvoice', 'InvoiceListResponse', 'CreateInvoiceRequest', 'UpdateInvoiceRequest', '/finance-invoices'),
    'sales_price_service.rs': ('SalesPrice', 'Vec<SalesPrice>', 'CreateSalesPriceRequest', 'UpdateSalesPriceRequest', '/sales/prices'),
    'batch_service.rs': ('Batch', 'Vec<Batch>', 'CreateBatchRequest', 'UpdateBatchRequest', '/batches'),
    'quality_inspection_service.rs': ('InspectionStandard', 'Vec<InspectionStandard>', 'CreateInspectionStandardRequest', 'CreateInspectionStandardRequest', '/quality-standards'),
    'budget_management_service.rs': ('BudgetItem', 'BudgetItemListResponse', 'CreateBudgetItemRequest', 'UpdateBudgetItemRequest', '/budgets/items'),
    'inventory_count_service.rs': ('InventoryCountDetail', 'Vec<InventoryCount>', 'CreateInventoryCountRequest', 'UpdateInventoryCountRequest', '/inventory/counts'),
    'ap_invoice_service.rs': ('ApInvoice', 'ApInvoiceListResponse', 'CreateApInvoiceRequest', 'UpdateApInvoiceRequest', '/ap/invoices'),
    'purchase_inspection_service.rs': ('PurchaseInspection', 'InspectionListResponse', 'CreatePurchaseInspectionRequest', 'UpdatePurchaseInspectionRequest', '/purchase-inspections'),
    'financial_analysis_service.rs': ('FinancialIndicator', 'Vec<FinancialIndicator>', 'CreateIndicatorRequest', 'UpdateIndicatorRequest', '/financial-analysis/ratios'),
    'dye_recipe_service.rs': ('DyeRecipe', 'DyeRecipeListResponse', 'CreateDyeRecipeRequest', 'UpdateDyeRecipeRequest', '/dye-recipe'),
    'inventory_transfer_service.rs': ('InventoryTransferDetail', 'Vec<InventoryTransfer>', 'CreateInventoryTransferRequest', 'UpdateInventoryTransferRequest', '/inventory/transfers'),
    'finance_payment_service.rs': ('FinancePayment', 'PaymentListResponse', 'CreatePaymentRequest', 'UpdatePaymentRequest', '/finance-payments'),
    'purchase_receipt_service.rs': ('PurchaseReceipt', 'PaginatedReceipts', 'CreatePurchaseReceiptRequest', 'UpdatePurchaseReceiptRequest', '/purchases/receipts'),
    'ar_invoice_service.rs': ('ArInvoice', 'ArInvoiceListResponse', 'CreateArInvoiceRequest', 'UpdateArInvoiceRequest', '/ar-invoices')
}

SERVICES_DIR = '/home/root0/桌面/121/1/frontend/src/services'

for filename, (model, list_res, create_req, update_req, base_path) in services.items():
    filepath = os.path.join(SERVICES_DIR, filename)
    with open(filepath, 'r') as f:
        content = f.read()
        
    if 'impl CrudService for' in content:
        continue
        
    # Get service name
    service_match = re.search(r'pub struct (\w+);', content)
    if not service_match:
        continue
    service_name = service_match.group(1)

    # regex to remove standard methods
    content = re.sub(rf'(?:\s*pub async fn list\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn list_{model.lower()}s\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn get\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn get_{model.lower()}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn create\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn create_{model.lower()}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn update\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn update_{model.lower()}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn delete\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn delete_{model.lower()}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    
    # Check if impl ServiceName is now empty
    content = re.sub(rf'impl {service_name} {{\s*}}', '', content)
    
    # Add imports
    if 'use crate::services::crud_service::CrudService;' not in content:
        content = content.replace('use crate::services::api::ApiService;', 'use crate::services::api::ApiService;\nuse crate::services::crud_service::CrudService;')
        if 'ApiService;' not in content:
            content = 'use crate::services::crud_service::CrudService;\n' + content

    # Prepend the CrudService implementation
    struct_def = f"pub struct {service_name};"
    impl_crud = f"""
impl CrudService for {service_name} {{
    type Model = {model};
    type ListResponse = {list_res};
    type CreateRequest = {create_req};
    type UpdateRequest = {update_req};

    fn base_path() -> &'static str {{
        "{base_path}"
    }}
}}
"""
    content = content.replace(struct_def, struct_def + "\n" + impl_crud)

    with open(filepath, 'w') as f:
        f.write(content)
    print(f"Updated {filename}")
