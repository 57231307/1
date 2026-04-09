use crate::pages::{
    customer_statement::CustomerStatementPage,
    sales_delivery::SalesDeliveryPage,
    inventory_reservation::InventoryReservationPage,
    operation_log::OperationLogPage,
    ApInvoicePage,
    ApVerificationPage,
    ArInvoicePage,
    ArReceiptPage,
    ArVerificationPage,
    CustomerPage,
    DashboardPage,
    DepartmentListPage,
    InitPage,
    InventoryAdjustmentPage,
    InventoryCountPage,
    InventoryStockPage,
    InventoryTransferPage,
    LoginPage,
    ProductCategoryPage,
    ProductListPage,
    PurchaseOrderPage,
    PurchaseReceiptPage,
    PurchaseReturnPage,
    RoleListPage,
    SalesOrderPage,
    SalesReturnPage,
    SupplierPage,
    UserListPage,
    WarehouseListPage
};
use crate::utils::storage::Storage;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/sales-deliveries")]
    SalesDeliveries,
    #[at("/inventory-reservations")]
    InventoryReservations,
    #[at("/customer-statement")]
    CustomerStatement,
    #[at("/operation-logs")]
    OperationLogs,
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
    #[at("/inventory-transfers")]
    Transfers,
    #[at("/inventory-counts")]
    Counts,
    #[at("/sales-returns")]
    SalesReturns,
    #[at("/customers")]
    Customers,
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
    #[at("/ap-invoices")]
    ApInvoices,
    #[at("/ap-verifications")]
    ApVerifications,
    #[at("/ar-invoices")]
    ArInvoices,
    #[at("/ar-verifications")]
    ArVerifications,
    #[at("/ar-receipts")]
    ArReceipts,
    #[at("/color-cards")]
    ColorCard,
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
        Route::Transfers => protected_route(|| html! { <InventoryTransferPage /> }),
        Route::ColorCard => html! { <crate::pages::color_card::ColorCardPage /> },
        Route::Counts => protected_route(|| html! { <InventoryCountPage /> }),
        Route::SalesReturns => protected_route(|| html! { <SalesReturnPage /> }),
        Route::Customers => protected_route(|| html! { <CustomerPage /> }),
        Route::PurchaseOrders => protected_route(|| html! { <PurchaseOrderPage /> }),
        Route::PurchaseReceipts => protected_route(|| html! { <PurchaseReceiptPage /> }),
        Route::PurchaseReturns => protected_route(|| html! { <PurchaseReturnPage /> }),
        Route::Suppliers => protected_route(|| html! { <SupplierPage /> }),
        Route::InventoryAdjustments => protected_route(|| html! { <InventoryAdjustmentPage /> }),
        Route::ApInvoices => protected_route(|| html! { <ApInvoicePage /> }),
        Route::ApVerifications => protected_route(|| html! { <ApVerificationPage /> }),
        Route::ArInvoices => protected_route(|| html! { <ArInvoicePage /> }),
        Route::ArVerifications => protected_route(|| html! { <ArVerificationPage /> }),
        Route::ArReceipts => protected_route(|| html! { <ArReceiptPage /> }),
        Route::SalesDeliveries => protected_route(|| html! { <SalesDeliveryPage /> }),
        Route::InventoryReservations => protected_route(|| html! { <InventoryReservationPage /> }),
        Route::CustomerStatement => protected_route(|| html! { <CustomerStatementPage /> }),
        Route::OperationLogs => protected_route(|| html! { <OperationLogPage /> }),
        Route::NotFound => html! {
            <crate::components::main_layout::MainLayout current_page={"404"}>
                <div class="p-8 text-center text-gray-500 text-xl">
                    {"404 - 页面未找到"}
                </div>
            </crate::components::main_layout::MainLayout>
        },
    }
}
