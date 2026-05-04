use yew::prelude::*;
use yew_router::prelude::*;
use crate::pages::{SystemSettingsPage, LoginPage, InitPage, DashboardPage, UserListPage, RoleListPage, ProductListPage, ProductCategoryPage, WarehouseListPage, DepartmentListPage, InventoryStockPage, SalesOrderPage, InventoryTransferPage, InventoryCountPage, FinanceInvoicePage, FinancePaymentPage, PurchasePricePage, SalesPricePage, SalesReturnPage, SalesAnalysisPage, QualityInspectionPage, FinancialAnalysisPage, SupplierEvaluationPage, FabricOrderPage, CustomerPage, BatchPage, PurchaseOrderPage, PurchaseReceiptPage, PurchaseReturnPage, SupplierPage, InventoryAdjustmentPage, AccountSubjectPage, VoucherPage, FundManagementPage, FixedAssetPage, CustomerCreditPage, DualUnitConverterPage, FiveDimensionPage, BusinessTracePage, ApInvoicePage, ApPaymentRequestPage, ApPaymentPage, ApReconciliationPage, ApVerificationPage, ArInvoicePage, AssistAccountingPage, SalesContractPage, PurchaseContractPage, CostCollectionPage, ApReportPage, PurchaseInspectionPage, DyeBatchPage, DyeRecipePage, GreigeFabricPage, crm_lead::CrmLeadPage, crm_opportunity::CrmOpportunityPage};
use crate::utils::storage::Storage;
use crate::utils::permissions;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Init,
    #[at("/login")]
    Login,
    #[at("/dashboard")]
    Dashboard,
    #[at("/users")]
    Users,
    #[at("/system-settings")]
    SystemSettings,
    #[at("/roles")]
    Roles,
    #[at("/products")]
    Products,
    #[at("/product-categories")]
    ProductCategories,
    #[at("/warehouses")]
    Warehouses,
    #[at("/departments")]
    Departments,
    #[at("/inventory")]
    Inventory,
    #[at("/sales")]
    Sales,
    #[at("/sales/fabric")]
    FabricOrders,
    #[at("/sales-contracts")]
    SalesContracts,
    #[at("/inventory-transfers")]
    Transfers,
    #[at("/inventory-counts")]
    Counts,
    #[at("/finance-invoices")]
    FinanceInvoices,
    #[at("/finance-payments")]
    FinancePayments,
    #[at("/purchase-prices")]
    PurchasePrices,
    #[at("/sales-prices")]
    SalesPrices,
    #[at("/sales-returns")]
    SalesReturns,
    #[at("/sales-analysis")]
    SalesAnalysis,
    #[at("/quality-inspection")]
    QualityInspection,
    #[at("/financial-analysis")]
    FinancialAnalysis,
    #[at("/supplier-evaluation")]
    SupplierEvaluation,
    #[at("/customers")]
    Customers,
    #[at("/batches")]
    Batches,
    #[at("/purchase-orders")]
    PurchaseOrders,
    #[at("/purchase-receipts")]
    PurchaseReceipts,
    #[at("/purchase-returns")]
    PurchaseReturns,
    #[at("/suppliers")]
    Suppliers,
    #[at("/inventory-adjustments")]
    InventoryAdjustments,
    #[at("/account-subjects")]
    AccountSubjects,
    #[at("/vouchers")]
    Vouchers,
    #[at("/fund-management")]
    FundManagement,
    #[at("/fixed-assets")]
    FixedAssets,
    #[at("/customer-credits")]
    CustomerCredits,
    #[at("/dual-unit-converter")]
    DualUnitConverter,
    #[at("/five-dimensions")]
    FiveDimensions,
    #[at("/business-trace")]
    BusinessTrace,
    #[at("/ap-invoices")]
    ApInvoices,
    #[at("/ap-payment-requests")]
    ApPaymentRequests,
    #[at("/ap-payments")]
    ApPayments,
    #[at("/ap-reconciliations")]
    ApReconciliations,
    #[at("/ap-verifications")]
    ApVerifications,
    #[at("/ap-reports")]
    ApReports,
    #[at("/ar-invoices")]
    ArInvoices,
    #[at("/assist-accounting")]
    AssistAccounting,
    #[at("/purchase-contracts")]
    PurchaseContracts,
    #[at("/cost-collections")]
    CostCollections,
    #[at("/purchase-inspections")]
    PurchaseInspections,
    #[at("/dye-batches")]
    DyeBatches,
    #[at("/dye-recipes")]
    DyeRecipes,
    #[at("/greige-fabrics")]
    GreigeFabrics,
    #[at("/crm/leads")]
    CrmLeads,
    #[at("/crm/opportunities")]
    CrmOpportunities,
    #[at("/my-tasks")]
    MyTasks,
    #[not_found]
    #[at("/404")]
    NotFound,
}

pub struct App;

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <HashRouter>
                <Switch<Route> render={switch} />
            </HashRouter>
        }
    }
}

use crate::components::main_layout::MainLayout;

fn protected_route_with_permission<F>(component: F, page_name: &str, resource: &str, action: &str) -> Html
where
    F: FnOnce() -> Html,
{
    if Storage::get_token().is_some() {
        if permissions::has_permission(resource, action) {
            html! {
                <MainLayout current_page={page_name.to_string()}>
                    {component()}
                </MainLayout>
            }
        } else {
            html! {
                <MainLayout current_page={page_name.to_string()}>
                    <div class="error-page" style="padding: 20px; text-align: center;">
                        <h1>{"无权访问"}</h1>
                        <p>{"您没有权限访问此页面"}</p>
                    </div>
                </MainLayout>
            }
        }
    } else {
        html! { <Redirect<Route> to={Route::Login}/> }
    }
}

fn protected_route<F>(component: F, page_name: &str) -> Html
where
    F: FnOnce() -> Html,
{
    if Storage::get_token().is_some() {
        html! {
            <MainLayout current_page={page_name.to_string()}>
                {component()}
            </MainLayout>
        }
    } else {
        html! { <Redirect<Route> to={Route::Login}/> }
    }
}

fn switch(route: Route) -> Html {
    match route {
        Route::Init => html! { <InitPage /> },
        Route::Login => html! { <LoginPage /> },
        Route::Dashboard => protected_route(|| html! { <DashboardPage /> }, "dashboard"),
        Route::Users => protected_route_with_permission(|| html! { <UserListPage /> }, "users", "user", "read"),
        Route::SystemSettings => protected_route(|| html! { <SystemSettingsPage /> }, "system-settings"),
        Route::Roles => protected_route(|| html! { <RoleListPage /> }, "roles"),
        Route::Products => protected_route(|| html! { <ProductListPage /> }, "products"),
        Route::ProductCategories => protected_route(|| html! { <ProductCategoryPage /> }, "product-categories"),
        Route::Warehouses => protected_route(|| html! { <WarehouseListPage /> }, "warehouses"),
        Route::Departments => protected_route(|| html! { <DepartmentListPage /> }, "departments"),
        Route::Inventory => protected_route_with_permission(|| html! { <InventoryStockPage /> }, "inventory", "inventory_stock", "read"),
        Route::Sales => protected_route_with_permission(|| html! { <SalesOrderPage /> }, "sales", "sales_order", "read"),
        Route::FabricOrders => protected_route(|| html! { <FabricOrderPage /> }, "fabric-orders"),
        Route::SalesContracts => protected_route(|| html! { <SalesContractPage /> }, "sales-contracts"),
        Route::Transfers => protected_route(|| html! { <InventoryTransferPage /> }, "transfers"),
        Route::Counts => protected_route(|| html! { <InventoryCountPage /> }, "counts"),
        Route::FinanceInvoices => protected_route(|| html! { <FinanceInvoicePage /> }, "finance-invoices"),
        Route::FinancePayments => protected_route(|| html! { <FinancePaymentPage /> }, "finance-payments"),
        Route::PurchasePrices => protected_route(|| html! { <PurchasePricePage /> }, "purchase-prices"),
        Route::SalesPrices => protected_route(|| html! { <SalesPricePage /> }, "sales-prices"),
        Route::SalesReturns => protected_route(|| html! { <SalesReturnPage /> }, "sales-returns"),
        Route::SalesAnalysis => protected_route(|| html! { <SalesAnalysisPage /> }, "sales-analysis"),
        Route::QualityInspection => protected_route(|| html! { <QualityInspectionPage /> }, "quality-inspection"),
        Route::FinancialAnalysis => protected_route(|| html! { <FinancialAnalysisPage /> }, "financial-analysis"),
        Route::SupplierEvaluation => protected_route(|| html! { <SupplierEvaluationPage /> }, "supplier-evaluation"),
        Route::Customers => protected_route(|| html! { <CustomerPage /> }, "customers"),
        Route::Batches => protected_route(|| html! { <BatchPage /> }, "batches"),
        Route::PurchaseOrders => protected_route_with_permission(|| html! { <PurchaseOrderPage /> }, "purchase-orders", "purchase_order", "read"),
        Route::PurchaseReceipts => protected_route(|| html! { <PurchaseReceiptPage /> }, "purchase-receipts"),
        Route::PurchaseReturns => protected_route(|| html! { <PurchaseReturnPage /> }, "purchase-returns"),
        Route::Suppliers => protected_route(|| html! { <SupplierPage /> }, "suppliers"),
        Route::InventoryAdjustments => protected_route(|| html! { <InventoryAdjustmentPage /> }, "inventory-adjustments"),
        Route::AccountSubjects => protected_route(|| html! { <AccountSubjectPage /> }, "account-subjects"),
        Route::Vouchers => protected_route(|| html! { <VoucherPage /> }, "vouchers"),
        Route::FundManagement => protected_route(|| html! { <FundManagementPage /> }, "fund-management"),
        Route::FixedAssets => protected_route(|| html! { <FixedAssetPage /> }, "fixed-assets"),
        Route::CustomerCredits => protected_route(|| html! { <CustomerCreditPage /> }, "customer-credits"),
        Route::DualUnitConverter => protected_route(|| html! { <DualUnitConverterPage /> }, "dual-unit-converter"),
        Route::FiveDimensions => protected_route(|| html! { <FiveDimensionPage /> }, "five-dimensions"),
        Route::BusinessTrace => protected_route(|| html! { <BusinessTracePage /> }, "business-trace"),
        Route::ApInvoices => protected_route_with_permission(|| html! { <ApInvoicePage /> }, "ap-invoices", "ap_invoice", "read"),
        Route::ApPaymentRequests => protected_route(|| html! { <ApPaymentRequestPage /> }, "ap-payment-requests"),
        Route::ApPayments => protected_route(|| html! { <ApPaymentPage /> }, "ap-payments"),
        Route::ApReconciliations => protected_route(|| html! { <ApReconciliationPage /> }, "ap-reconciliations"),
        Route::ApVerifications => protected_route(|| html! { <ApVerificationPage /> }, "ap-verifications"),
        Route::ApReports => protected_route(|| html! { <ApReportPage /> }, "ap-reports"),
        Route::ArInvoices => protected_route(|| html! { <ArInvoicePage /> }, "ar-invoices"),
        Route::AssistAccounting => protected_route(|| html! { <AssistAccountingPage /> }, "assist-accounting"),
        Route::PurchaseContracts => protected_route(|| html! { <PurchaseContractPage /> }, "purchase-contracts"),
        Route::CostCollections => protected_route(|| html! { <CostCollectionPage /> }, "cost-collections"),
        Route::PurchaseInspections => protected_route(|| html! { <PurchaseInspectionPage /> }, "purchase-inspections"),
        Route::DyeBatches => protected_route(|| html! { <DyeBatchPage /> }, "dye-batches"),
        Route::DyeRecipes => protected_route(|| html! { <DyeRecipePage /> }, "dye-recipes"),
        Route::GreigeFabrics => protected_route(|| html! { <GreigeFabricPage /> }, "greige-fabrics"),
        Route::CrmLeads => protected_route(|| html! { <CrmLeadPage /> }, "crm-leads"),
        Route::CrmOpportunities => protected_route(|| html! { <CrmOpportunityPage /> }, "crm-opportunities"),
        Route::MyTasks => protected_route(|| html! { <crate::pages::my_tasks::MyTasksPage /> }, "my-tasks"),
        Route::NotFound => html! { <div>{"页面未找到"}</div> },
    }
}
