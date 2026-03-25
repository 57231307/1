use yew::prelude::*;
use yew_router::prelude::*;
use crate::pages::{LoginPage, InitPage, DashboardPage, UserListPage, RoleListPage, ProductListPage, ProductCategoryPage, WarehouseListPage, DepartmentListPage, InventoryStockPage, SalesOrderPage, InventoryTransferPage, InventoryCountPage, FinanceInvoicePage, FinancePaymentPage, PurchasePricePage, SalesPricePage, SalesAnalysisPage, QualityInspectionPage, FinancialAnalysisPage, SupplierEvaluationPage, FabricOrderPage, CustomerPage, BatchPage, PurchaseOrderPage, PurchaseReceiptPage, PurchaseReturnPage, SupplierPage, InventoryAdjustmentPage, AccountSubjectPage, VoucherPage, FundManagementPage, FixedAssetPage, CustomerCreditPage, DualUnitConverterPage, FiveDimensionPage, BusinessTracePage, ApInvoicePage, ApPaymentRequestPage, ApPaymentPage, ApReconciliationPage, ApVerificationPage, ArInvoicePage, AssistAccountingPage, SalesContractPage, PurchaseContractPage, CostCollectionPage, ApReportPage, PurchaseInspectionPage};

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

fn switch(route: Route) -> Html {
    match route {
        Route::Init => html! { <InitPage /> },
        Route::Login => html! { <LoginPage /> },
        Route::Dashboard => html! { <DashboardPage /> },
        Route::Users => html! { <UserListPage /> },
        Route::Roles => html! { <RoleListPage /> },
        Route::Products => html! { <ProductListPage /> },
        Route::ProductCategories => html! { <ProductCategoryPage /> },
        Route::Warehouses => html! { <WarehouseListPage /> },
        Route::Departments => html! { <DepartmentListPage /> },
        Route::Inventory => html! { <InventoryStockPage /> },
        Route::Sales => html! { <SalesOrderPage /> },
        Route::FabricOrders => html! { <FabricOrderPage /> },
        Route::SalesContracts => html! { <SalesContractPage /> },
        Route::Transfers => html! { <InventoryTransferPage /> },
        Route::Counts => html! { <InventoryCountPage /> },
        Route::FinanceInvoices => html! { <FinanceInvoicePage /> },
        Route::FinancePayments => html! { <FinancePaymentPage /> },
        Route::PurchasePrices => html! { <PurchasePricePage /> },
        Route::SalesPrices => html! { <SalesPricePage /> },
        Route::SalesAnalysis => html! { <SalesAnalysisPage /> },
        Route::QualityInspection => html! { <QualityInspectionPage /> },
        Route::FinancialAnalysis => html! { <FinancialAnalysisPage /> },
        Route::SupplierEvaluation => html! { <SupplierEvaluationPage /> },
        Route::Customers => html! { <CustomerPage /> },
        Route::Batches => html! { <BatchPage /> },
        Route::PurchaseOrders => html! { <PurchaseOrderPage /> },
        Route::PurchaseReceipts => html! { <PurchaseReceiptPage /> },
        Route::PurchaseReturns => html! { <PurchaseReturnPage /> },
        Route::Suppliers => html! { <SupplierPage /> },
        Route::InventoryAdjustments => html! { <InventoryAdjustmentPage /> },
        Route::AccountSubjects => html! { <AccountSubjectPage /> },
        Route::Vouchers => html! { <VoucherPage /> },
        Route::FundManagement => html! { <FundManagementPage /> },
        Route::FixedAssets => html! { <FixedAssetPage /> },
        Route::CustomerCredits => html! { <CustomerCreditPage /> },
        Route::DualUnitConverter => html! { <DualUnitConverterPage /> },
        Route::FiveDimensions => html! { <FiveDimensionPage /> },
        Route::BusinessTrace => html! { <BusinessTracePage /> },
        Route::ApInvoices => html! { <ApInvoicePage /> },
        Route::ApPaymentRequests => html! { <ApPaymentRequestPage /> },
        Route::ApPayments => html! { <ApPaymentPage /> },
        Route::ApReconciliations => html! { <ApReconciliationPage /> },
        Route::ApVerifications => html! { <ApVerificationPage /> },
        Route::ApReports => html! { <ApReportPage /> },
        Route::ArInvoices => html! { <ArInvoicePage /> },
        Route::AssistAccounting => html! { <AssistAccountingPage /> },
        Route::PurchaseContracts => html! { <PurchaseContractPage /> },
        Route::CostCollections => html! { <CostCollectionPage /> },
        Route::PurchaseInspections => html! { <PurchaseInspectionPage /> },
        Route::NotFound => html! { <div>{"页面未找到"}</div> },
    }
}
