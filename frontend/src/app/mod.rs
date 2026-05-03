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

fn protected_route_with_permission<F>(component: F, resource: &str, action: &str) -> Html
where
    F: FnOnce() -> Html,
{
    if Storage::get_token().is_some() {
        if permissions::has_permission(resource, action) {
            component()
        } else {
            html! {
                <div class="error-page" style="padding: 20px; text-align: center;">
                    <h1>{"无权访问"}</h1>
                    <p>{"您没有权限访问此页面"}</p>
                </div>
            }
        }
    } else {
        html! { <Redirect<Route> to={Route::Login}/> }
    }
}

fn protected_route<F>(component: F) -> Html
where
    F: FnOnce() -> Html,
{
    if Storage::get_token().is_some() {
        component()
    } else {
        html! { <Redirect<Route> to={Route::Login}/> }
    }
}

fn switch(route: Route) -> Html {
    match route {
        Route::Init => html! { <InitPage /> },
        Route::Login => html! { <LoginPage /> },
        Route::Dashboard => protected_route(|| html! { <DashboardPage /> }),
        Route::Users => protected_route_with_permission(|| html! { <UserListPage /> }, "user", "read"),
            Route::SystemSettings => protected_route(|| html! { <SystemSettingsPage /> }),
        Route::Roles => protected_route(|| html! { <RoleListPage /> }),
        Route::Products => protected_route(|| html! { <ProductListPage /> }),
        Route::ProductCategories => protected_route(|| html! { <ProductCategoryPage /> }),
        Route::Warehouses => protected_route(|| html! { <WarehouseListPage /> }),
        Route::Departments => protected_route(|| html! { <DepartmentListPage /> }),
        Route::Inventory => protected_route_with_permission(|| html! { <InventoryStockPage /> }, "inventory_stock", "read"),
        Route::Sales => protected_route_with_permission(|| html! { <SalesOrderPage /> }, "sales_order", "read"),
        Route::FabricOrders => protected_route(|| html! { <FabricOrderPage /> }),
        Route::SalesContracts => protected_route(|| html! { <SalesContractPage /> }),
        Route::Transfers => protected_route(|| html! { <InventoryTransferPage /> }),
        Route::Counts => protected_route(|| html! { <InventoryCountPage /> }),
        Route::FinanceInvoices => protected_route(|| html! { <FinanceInvoicePage /> }),
        Route::FinancePayments => protected_route(|| html! { <FinancePaymentPage /> }),
        Route::PurchasePrices => protected_route(|| html! { <PurchasePricePage /> }),
        Route::SalesPrices => protected_route(|| html! { <SalesPricePage /> }),
        Route::SalesReturns => protected_route(|| html! { <SalesReturnPage /> }),
        Route::SalesAnalysis => protected_route(|| html! { <SalesAnalysisPage /> }),
        Route::QualityInspection => protected_route(|| html! { <QualityInspectionPage /> }),
        Route::FinancialAnalysis => protected_route(|| html! { <FinancialAnalysisPage /> }),
        Route::SupplierEvaluation => protected_route(|| html! { <SupplierEvaluationPage /> }),
        Route::Customers => protected_route(|| html! { <CustomerPage /> }),
        Route::Batches => protected_route(|| html! { <BatchPage /> }),
        Route::PurchaseOrders => protected_route_with_permission(|| html! { <PurchaseOrderPage /> }, "purchase_order", "read"),
        Route::PurchaseReceipts => protected_route(|| html! { <PurchaseReceiptPage /> }),
        Route::PurchaseReturns => protected_route(|| html! { <PurchaseReturnPage /> }),
        Route::Suppliers => protected_route(|| html! { <SupplierPage /> }),
        Route::InventoryAdjustments => protected_route(|| html! { <InventoryAdjustmentPage /> }),
        Route::AccountSubjects => protected_route(|| html! { <AccountSubjectPage /> }),
        Route::Vouchers => protected_route(|| html! { <VoucherPage /> }),
        Route::FundManagement => protected_route(|| html! { <FundManagementPage /> }),
        Route::FixedAssets => protected_route(|| html! { <FixedAssetPage /> }),
        Route::CustomerCredits => protected_route(|| html! { <CustomerCreditPage /> }),
        Route::DualUnitConverter => protected_route(|| html! { <DualUnitConverterPage /> }),
        Route::FiveDimensions => protected_route(|| html! { <FiveDimensionPage /> }),
        Route::BusinessTrace => protected_route(|| html! { <BusinessTracePage /> }),
        Route::ApInvoices => protected_route_with_permission(|| html! { <ApInvoicePage /> }, "ap_invoice", "read"),
        Route::ApPaymentRequests => protected_route(|| html! { <ApPaymentRequestPage /> }),
        Route::ApPayments => protected_route(|| html! { <ApPaymentPage /> }),
        Route::ApReconciliations => protected_route(|| html! { <ApReconciliationPage /> }),
        Route::ApVerifications => protected_route(|| html! { <ApVerificationPage /> }),
        Route::ApReports => protected_route(|| html! { <ApReportPage /> }),
        Route::ArInvoices => protected_route(|| html! { <ArInvoicePage /> }),
        Route::AssistAccounting => protected_route(|| html! { <AssistAccountingPage /> }),
        Route::PurchaseContracts => protected_route(|| html! { <PurchaseContractPage /> }),
        Route::CostCollections => protected_route(|| html! { <CostCollectionPage /> }),
        Route::PurchaseInspections => protected_route(|| html! { <PurchaseInspectionPage /> }),
        Route::DyeBatches => protected_route(|| html! { <DyeBatchPage /> }),
        Route::DyeRecipes => protected_route(|| html! { <DyeRecipePage /> }),
        Route::GreigeFabrics => protected_route(|| html! { <GreigeFabricPage /> }),
        Route::CrmLeads => protected_route(|| html! { <CrmLeadPage /> }),
        Route::CrmOpportunities => protected_route(|| html! { <CrmOpportunityPage /> }),
        Route::MyTasks => protected_route(|| html! { <crate::pages::my_tasks::MyTasksPage /> }),
        Route::NotFound => html! { <div>{"页面未找到"}</div> },
    }
}
