use crate::pages::{
    crm_lead::CrmLeadPage, crm_opportunity::CrmOpportunityPage, AccountSubjectPage, ApInvoicePage,
    ApPaymentPage, ApPaymentRequestPage, ApReconciliationPage, ApReportPage, ApVerificationPage,
    ArInvoicePage, ArReceiptPage, ArVerificationPage, AssistAccountingPage, BatchPage, BudgetManagementPage, BusinessTracePage, CostCollectionPage,
    CustomerCreditPage, CustomerPage, DashboardPage, DepartmentListPage, DualUnitConverterPage,
    DyeBatchPage, DyeRecipePage, FabricOrderPage, FinanceInvoicePage, FinancePaymentPage,
    FinancialAnalysisPage, FiveDimensionPage, FixedAssetPage, FundManagementPage, GreigeFabricPage,
    InitPage, InventoryAdjustmentPage, InventoryCountPage, InventoryStockPage,
    InventoryTransferPage, LoginPage, ProductCategoryPage, ProductListPage, PurchaseContractPage,
    PurchaseInspectionPage, PurchaseOrderPage, PurchasePricePage, PurchaseReceiptPage,
    PurchaseReturnPage, QualityInspectionPage, RoleListPage, SalesAnalysisPage, SalesContractPage,
    SalesOrderPage, SalesPricePage, SalesReturnPage, SupplierEvaluationPage, SupplierPage,
    UserListPage, VoucherPage, WarehouseListPage,
};
use crate::utils::storage::Storage;
use yew::prelude::*;
use yew_router::prelude::*;

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
    #[at("/ar-verifications")]
    ArVerifications,
    #[at("/ar-receipts")]
    ArReceipts,
    #[at("/budget-management")]
    BudgetManagement,
    #[not_found]
    #[at("/404/*path")]
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
        Route::Users => protected_route(|| html! { <UserListPage /> }),
        Route::Roles => protected_route(|| html! { <RoleListPage /> }),
        Route::Products => protected_route(|| html! { <ProductListPage /> }),
        Route::ProductCategories => protected_route(|| html! { <ProductCategoryPage /> }),
        Route::Warehouses => protected_route(|| html! { <WarehouseListPage /> }),
        Route::Departments => protected_route(|| html! { <DepartmentListPage /> }),
        Route::Inventory => protected_route(|| html! { <InventoryStockPage /> }),
        Route::Sales => protected_route(|| html! { <SalesOrderPage /> }),
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
        Route::PurchaseOrders => protected_route(|| html! { <PurchaseOrderPage /> }),
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
        Route::ApInvoices => protected_route(|| html! { <ApInvoicePage /> }),
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
        Route::ArVerifications => protected_route(|| html! { <ArVerificationPage /> }),
        Route::ArReceipts => protected_route(|| html! { <ArReceiptPage /> }),
        Route::BudgetManagement => protected_route(|| html! { <BudgetManagementPage /> }),
        Route::NotFound => html! {
            <crate::components::main_layout::MainLayout current_page={"404"}>
                <div class="p-8 text-center text-gray-500 text-xl">
                    {"404 - 页面未找到"}
                </div>
            </crate::components::main_layout::MainLayout>
        },
    }
}
