use yew::prelude::*;
use yew_router::prelude::*;
use crate::app::Route;

#[derive(Properties, PartialEq)]
pub struct NavigationProps {
    pub current_page: String,
}

#[function_component(Navigation)]
pub fn navigation(props: &NavigationProps) -> Html {
    let navigator = use_navigator();

    let on_dashboard = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Dashboard);
            }
        })
    };

    let on_users = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Users);
            }
        })
    };

    let on_roles = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Roles);
            }
        })
    };

    let on_products = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Products);
            }
        })
    };

    let on_product_categories = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ProductCategories);
            }
        })
    };

    let on_warehouses = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Warehouses);
            }
        })
    };

    let on_departments = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Departments);
            }
        })
    };

    let on_inventory = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Inventory);
            }
        })
    };

    let on_sales = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Sales);
            }
        })
    };

    let on_transfers = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Transfers);
            }
        })
    };

    let on_counts = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Counts);
            }
        })
    };

    let on_finance_invoices = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::FinanceInvoices);
            }
        })
    };

    let on_finance_payments = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::FinancePayments);
            }
        })
    };

    let on_purchase_prices = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::PurchasePrices);
            }
        })
    };

    let on_sales_prices = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::SalesPrices);
            }
        })
    };

    let on_sales_analysis = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::SalesAnalysis);
            }
        })
    };

    let on_quality_inspection = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::QualityInspection);
            }
        })
    };

    let on_financial_analysis = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::FinancialAnalysis);
            }
        })
    };

    let on_supplier_evaluation = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::SupplierEvaluation);
            }
        })
    };

    let on_fabric_orders = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::FabricOrders);
            }
        })
    };

    let on_customers = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Customers);
            }
        })
    };

    let on_batches = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Batches);
            }
        })
    };

    let on_purchase_orders = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::PurchaseOrders);
            }
        })
    };

    let on_purchase_receipts = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::PurchaseReceipts);
            }
        })
    };

    let on_purchase_returns = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::PurchaseReturns);
            }
        })
    };

    let on_suppliers = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Suppliers);
            }
        })
    };

    let on_inventory_adjustments = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::InventoryAdjustments);
            }
        })
    };

    let on_account_subjects = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::AccountSubjects);
            }
        })
    };

    let on_fund_management = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::FundManagement);
            }
        })
    };

    let on_fixed_assets = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::FixedAssets);
            }
        })
    };

    let on_customer_credits = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::CustomerCredits);
            }
        })
    };

    let on_dual_unit_converter = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::DualUnitConverter);
            }
        })
    };

    let on_five_dimensions = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::FiveDimensions);
            }
        })
    };

    let on_ap_invoices = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ApInvoices);
            }
        })
    };

    let on_ap_payment_requests = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ApPaymentRequests);
            }
        })
    };

    let on_ap_payments = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ApPayments);
            }
        })
    };

    let on_ap_reconciliations = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ApReconciliations);
            }
        })
    };

    let on_ap_verifications = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ApVerifications);
            }
        })
    };

    let on_ap_reports = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ApReports);
            }
        })
    };

    let on_ar_invoices = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::ArInvoices);
            }
        })
    };

    let on_assist_accounting = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::AssistAccounting);
            }
        })
    };

    let on_sales_contracts = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::SalesContracts);
            }
        })
    };

    let on_purchase_contracts = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::PurchaseContracts);
            }
        })
    };

    let on_cost_collections = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::CostCollections);
            }
        })
    };

    let on_purchase_inspections = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::PurchaseInspections);
            }
        })
    };

    html! {
        <nav class="navigation">
            <div class="nav-brand">{"面料 ERP 系统"}</div>
            <div class="nav-menu">
                <a class={if props.current_page == "dashboard" { "nav-item active" } else { "nav-item" }} onclick={on_dashboard}>
                    {"仪表盘"}
                </a>
                <a class={if props.current_page == "users" { "nav-item active" } else { "nav-item" }} onclick={on_users}>
                    {"用户管理"}
                </a>
                <a class={if props.current_page == "roles" { "nav-item active" } else { "nav-item" }} onclick={on_roles}>
                    {"角色管理"}
                </a>
                <a class={if props.current_page == "products" { "nav-item active" } else { "nav-item" }} onclick={on_products}>
                    {"产品管理"}
                </a>
                <a class={if props.current_page == "product_categories" { "nav-item active" } else { "nav-item" }} onclick={on_product_categories}>
                    {"产品类别"}
                </a>
                <a class={if props.current_page == "warehouses" { "nav-item active" } else { "nav-item" }} onclick={on_warehouses}>
                    {"仓库管理"}
                </a>
                <a class={if props.current_page == "departments" { "nav-item active" } else { "nav-item" }} onclick={on_departments}>
                    {"部门管理"}
                </a>
                <a class={if props.current_page == "inventory" { "nav-item active" } else { "nav-item" }} onclick={on_inventory}>
                    {"库存管理"}
                </a>
                <a class={if props.current_page == "sales" { "nav-item active" } else { "nav-item" }} onclick={on_sales}>
                    {"销售订单"}
                </a>
                <a class={if props.current_page == "transfers" { "nav-item active" } else { "nav-item" }} onclick={on_transfers}>
                    {"库存调拨"}
                </a>
                <a class={if props.current_page == "counts" { "nav-item active" } else { "nav-item" }} onclick={on_counts}>
                    {"库存盘点"}
                </a>
                <a class={if props.current_page == "finance_invoices" { "nav-item active" } else { "nav-item" }} onclick={on_finance_invoices}>
                    {"发票管理"}
                </a>
                <a class={if props.current_page == "finance_payments" { "nav-item active" } else { "nav-item" }} onclick={on_finance_payments}>
                    {"付款管理"}
                </a>
                <a class={if props.current_page == "purchase_prices" { "nav-item active" } else { "nav-item" }} onclick={on_purchase_prices}>
                    {"采购价格"}
                </a>
                <a class={if props.current_page == "sales_prices" { "nav-item active" } else { "nav-item" }} onclick={on_sales_prices}>
                    {"销售价格"}
                </a>
                <a class={if props.current_page == "sales_analysis" { "nav-item active" } else { "nav-item" }} onclick={on_sales_analysis}>
                    {"销售分析"}
                </a>
                <a class={if props.current_page == "quality_inspection" { "nav-item active" } else { "nav-item" }} onclick={on_quality_inspection}>
                    {"质量检验"}
                </a>
                <a class={if props.current_page == "financial_analysis" { "nav-item active" } else { "nav-item" }} onclick={on_financial_analysis}>
                    {"财务分析"}
                </a>
                <a class={if props.current_page == "supplier_evaluation" { "nav-item active" } else { "nav-item" }} onclick={on_supplier_evaluation}>
                    {"供应商评估"}
                </a>
                <a class={if props.current_page == "fabric_orders" { "nav-item active" } else { "nav-item" }} onclick={on_fabric_orders}>
                    {"面料订单"}
                </a>
                <a class={if props.current_page == "customers" { "nav-item active" } else { "nav-item" }} onclick={on_customers}>
                    {"客户管理"}
                </a>
                <a class={if props.current_page == "batches" { "nav-item active" } else { "nav-item" }} onclick={on_batches}>
                    {"批次管理"}
                </a>
                <a class={if props.current_page == "purchase_orders" { "nav-item active" } else { "nav-item" }} onclick={on_purchase_orders}>
                    {"采购订单"}
                </a>
                <a class={if props.current_page == "purchase_receipts" { "nav-item active" } else { "nav-item" }} onclick={on_purchase_receipts}>
                    {"采购收货"}
                </a>
                <a class={if props.current_page == "purchase_returns" { "nav-item active" } else { "nav-item" }} onclick={on_purchase_returns}>
                    {"采购退货"}
                </a>
                <a class={if props.current_page == "suppliers" { "nav-item active" } else { "nav-item" }} onclick={on_suppliers}>
                    {"供应商管理"}
                </a>
                <a class={if props.current_page == "inventory_adjustments" { "nav-item active" } else { "nav-item" }} onclick={on_inventory_adjustments}>
                    {"库存调整"}
                </a>
                <a class={if props.current_page == "account_subjects" { "nav-item active" } else { "nav-item" }} onclick={on_account_subjects}>
                    {"会计科目"}
                </a>
                <a class={if props.current_page == "fund_management" { "nav-item active" } else { "nav-item" }} onclick={on_fund_management}>
                    {"资金管理"}
                </a>
                <a class={if props.current_page == "fixed_assets" { "nav-item active" } else { "nav-item" }} onclick={on_fixed_assets}>
                    {"固定资产"}
                </a>
                <a class={if props.current_page == "customer_credits" { "nav-item active" } else { "nav-item" }} onclick={on_customer_credits}>
                    {"客户信用"}
                </a>
                <a class={if props.current_page == "dual_unit_converter" { "nav-item active" } else { "nav-item" }} onclick={on_dual_unit_converter}>
                    {"双计量单位转换"}
                </a>
                <a class={if props.current_page == "five_dimensions" { "nav-item active" } else { "nav-item" }} onclick={on_five_dimensions}>
                    {"五维查询"}
                </a>
                <a class={if props.current_page == "ap_invoices" { "nav-item active" } else { "nav-item" }} onclick={on_ap_invoices}>
                    {"应付发票"}
                </a>
                <a class={if props.current_page == "ap_payment_requests" { "nav-item active" } else { "nav-item" }} onclick={on_ap_payment_requests}>
                    {"付款申请"}
                </a>
                <a class={if props.current_page == "ap_payments" { "nav-item active" } else { "nav-item" }} onclick={on_ap_payments}>
                    {"付款管理"}
                </a>
                <a class={if props.current_page == "ap_reconciliations" { "nav-item active" } else { "nav-item" }} onclick={on_ap_reconciliations}>
                    {"应付对账"}
                </a>
                <a class={if props.current_page == "ap_verifications" { "nav-item active" } else { "nav-item" }} onclick={on_ap_verifications}>
                    {"应付核销"}
                </a>
                <a class={if props.current_page == "ap_reports" { "nav-item active" } else { "nav-item" }} onclick={on_ap_reports}>
                    {"应付报表"}
                </a>
                <a class={if props.current_page == "ar_invoices" { "nav-item active" } else { "nav-item" }} onclick={on_ar_invoices}>
                    {"应收发票"}
                </a>
                <a class={if props.current_page == "assist_accounting" { "nav-item active" } else { "nav-item" }} onclick={on_assist_accounting}>
                    {"辅助核算"}
                </a>
                <a class={if props.current_page == "sales_contracts" { "nav-item active" } else { "nav-item" }} onclick={on_sales_contracts}>
                    {"销售合同"}
                </a>
                <a class={if props.current_page == "purchase_contracts" { "nav-item active" } else { "nav-item" }} onclick={on_purchase_contracts}>
                    {"采购合同"}
                </a>
                <a class={if props.current_page == "cost_collections" { "nav-item active" } else { "nav-item" }} onclick={on_cost_collections}>
                    {"成本归集"}
                </a>
                <a class={if props.current_page == "purchase_inspections" { "nav-item active" } else { "nav-item" }} onclick={on_purchase_inspections}>
                    {"采购检验"}
                </a>
            </div>
        </nav>
    }
}
