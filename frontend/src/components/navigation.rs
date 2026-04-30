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

    // 一级折叠状态
    let l1_dashboard = use_state(|| true);
    let l1_basic = use_state(|| false);
    let l1_supply = use_state(|| false);
    let l1_inventory = use_state(|| false);
    let l1_finance = use_state(|| false);
    let l1_fabric = use_state(|| false);
    let l1_system = use_state(|| false);

    // 二级折叠状态
    let l2_basic_org = use_state(|| false);
    let l2_basic_product = use_state(|| false);
    let l2_basic_contact = use_state(|| false);
    let l2_basic_wh = use_state(|| false);
    
    let l2_supply_sales = use_state(|| false);
    let l2_supply_purchase = use_state(|| false);
    let l2_supply_price = use_state(|| false);
    
    let l2_inv_ops = use_state(|| false);
    let l2_inv_qa = use_state(|| false);
    
    let l2_fin_acc = use_state(|| false);
    let l2_fin_ar_ap = use_state(|| false);
    let l2_fin_cost = use_state(|| false);
    
    let l2_fab_prod = use_state(|| false);
    let l2_fab_tools = use_state(|| false);
    
    let l2_sys_report = use_state(|| false);
    let l2_sys_admin = use_state(|| false);

    // 路由跳转辅助宏
    macro_rules! nav_cb {
        ($route:expr) => {{
            let navigator = navigator.clone();
            Callback::from(move |_| {
                if let Some(nav) = &navigator {
                    nav.push(&$route);
                }
            })
        }};
    }

    let on_dashboard = nav_cb!(Route::Dashboard);
    
    // 基础数据
    let on_users = nav_cb!(Route::Users);
    let on_roles = nav_cb!(Route::Roles);
    let on_departments = nav_cb!(Route::Departments);
    let on_products = nav_cb!(Route::Products);
    let on_categories = nav_cb!(Route::ProductCategories);
    let on_customers = nav_cb!(Route::Customers);
    let on_suppliers = nav_cb!(Route::Suppliers);
    let on_warehouses = nav_cb!(Route::Warehouses);

    // 供应链
    let on_sales_orders = nav_cb!(Route::Sales);
    let on_fabric_orders = nav_cb!(Route::FabricOrders);
    let on_sales_contracts = nav_cb!(Route::SalesContracts);
    let on_sales_returns = nav_cb!(Route::SalesReturns);
    
    let on_po = nav_cb!(Route::PurchaseOrders);
    let on_pc = nav_cb!(Route::PurchaseContracts);
    let on_pr = nav_cb!(Route::PurchaseReceipts);
    let on_pret = nav_cb!(Route::PurchaseReturns);
    
    let on_sales_prices = nav_cb!(Route::SalesPrices);
    let on_purchase_prices = nav_cb!(Route::PurchasePrices);

    // 仓储质量
    let on_inv_stock = nav_cb!(Route::Inventory);
    let on_inv_transfer = nav_cb!(Route::Transfers);
    let on_inv_count = nav_cb!(Route::Counts);
    let on_inv_adj = nav_cb!(Route::InventoryAdjustments);
    
    let on_qa_insp = nav_cb!(Route::QualityInspection);
    let on_qa_po = nav_cb!(Route::PurchaseInspections);
    let on_qa_supp = nav_cb!(Route::SupplierEvaluation);

    // 财务
    let on_fin_fund = nav_cb!(Route::FundManagement);
    let on_fin_asset = nav_cb!(Route::FixedAssets);
    let on_fin_subj = nav_cb!(Route::AccountSubjects);
    let on_fin_voucher = nav_cb!(Route::Vouchers);
    
    let on_fin_ar_inv = nav_cb!(Route::ArInvoices);
    let on_fin_ap_inv = nav_cb!(Route::ApInvoices);
    let on_fin_ap_req = nav_cb!(Route::ApPaymentRequests);
    let on_fin_ap_pay = nav_cb!(Route::ApPayments);
    let on_fin_credit = nav_cb!(Route::CustomerCredits);
    
    let on_fin_cost = nav_cb!(Route::CostCollections);
    let on_fin_assist = nav_cb!(Route::AssistAccounting);

    // 面料特色
    let on_fab_batch = nav_cb!(Route::Batches);
    let on_fab_dye = nav_cb!(Route::DyeBatches);
    let on_fab_recipe = nav_cb!(Route::DyeRecipes);
    let on_fab_greige = nav_cb!(Route::GreigeFabrics);
    
    let on_fab_dual = nav_cb!(Route::DualUnitConverter);
    let on_fab_five = nav_cb!(Route::FiveDimensions);
    let on_fab_trace = nav_cb!(Route::BusinessTrace);

    // 系统分析
    let on_sys_sales_rep = nav_cb!(Route::SalesAnalysis);
    let on_sys_fin_rep = nav_cb!(Route::FinancialAnalysis);
    let on_sys_settings = nav_cb!(Route::SystemSettings);

    // 辅助函数：渲染三级菜单项
    let render_item = |label: &str, page_key: &str, cb: Callback<MouseEvent>| -> Html {
        let is_active = props.current_page == page_key;
        html! {
            <a class={if is_active { "nav-item active text-sm pl-8 py-2 block" } else { "nav-item text-sm pl-8 py-2 block hover:bg-gray-100" }} onclick={cb}>
                {label}
            </a>
        }
    };

    // 辅助函数：渲染二级菜单组
    let render_l2_group = |label: &str, state: UseStateHandle<bool>, items: Html| -> Html {
        let is_open = *state;
        let toggle = {
            let state = state.clone();
            Callback::from(move |_| state.set(!is_open))
        };
        html! {
            <div class="nav-l2-group border-l-2 border-gray-200 ml-4 pl-2 my-1">
                <div class="nav-l2-header cursor-pointer flex justify-between items-center py-2 text-sm font-medium text-gray-700 hover:text-indigo-600" onclick={toggle}>
                    <span>{label}</span>
                    <span class={if is_open { "transform rotate-180 transition-transform text-xs" } else { "transition-transform text-xs" }}>{"▼"}</span>
                </div>
                {if is_open {
                    html! { <div class="nav-l2-content flex flex-col space-y-1"> {items} </div> }
                } else {
                    html! {}
                }}
            </div>
        }
    };

    // 辅助函数：渲染一级菜单组
    let render_l1_group = |label: &str, state: UseStateHandle<bool>, content: Html| -> Html {
        let is_open = *state;
        let toggle = {
            let state = state.clone();
            Callback::from(move |_| state.set(!is_open))
        };
        html! {
            <div class="nav-group mb-2">
                <div class="nav-group-header cursor-pointer flex justify-between items-center px-4 py-3 bg-gray-50 rounded-md hover:bg-gray-100" onclick={toggle}>
                    <span class="nav-group-title font-bold text-gray-800">{label}</span>
                    <span class={if is_open { "nav-group-icon open transform rotate-180 transition-transform" } else { "nav-group-icon transition-transform" }}>{"▼"}</span>
                </div>
                {if is_open {
                    html! { <div class="nav-group-content mt-1"> {content} </div> }
                } else {
                    html! {}
                }}
            </div>
        }
    };

    html! {
        <nav class="navigation w-64 h-screen overflow-y-auto bg-white border-r border-gray-200 shadow-sm flex flex-col">
            <div class="nav-brand px-6 py-5 text-xl font-extrabold text-indigo-700 border-b border-gray-100 sticky top-0 bg-white z-10">
                {"秉羲面料管理"}
            </div>
            <div class="nav-menu flex-1 px-3 py-4 space-y-1">
                
                {render_l1_group("工作台", l1_dashboard, html! {
                    {render_item("首页", "dashboard", on_dashboard)}
                })}

                {render_l1_group("基础数据", l1_basic, html! {
                    <>
                        {render_l2_group("组织架构", l2_basic_org, html! {
                            <>
                                {render_item("用户管理", "users", on_users)}
                                {render_item("角色管理", "roles", on_roles)}
                                {render_item("部门管理", "departments", on_departments)}
                            </>
                        })}
                        {render_l2_group("产品资料", l2_basic_product, html! {
                            <>
                                {render_item("产品管理", "products", on_products)}
                                {render_item("产品类别", "product_categories", on_categories)}
                            </>
                        })}
                        {render_l2_group("业务往来", l2_basic_contact, html! {
                            <>
                                {render_item("客户管理", "customers", on_customers)}
                                {render_item("供应商管理", "suppliers", on_suppliers)}
                            </>
                        })}
                        {render_l2_group("仓库资料", l2_basic_wh, html! {
                            {render_item("仓库管理", "warehouses", on_warehouses)}
                        })}
                    </>
                })}

                {render_l1_group("供应链管理", l1_supply, html! {
                    <>
                        {render_l2_group("销售业务", l2_supply_sales, html! {
                            <>
                                {render_item("销售订单", "sales", on_sales_orders)}
                                {render_item("面料订单", "fabric_orders", on_fabric_orders)}
                                {render_item("销售合同", "sales_contracts", on_sales_contracts)}
                                {render_item("销售退货", "sales_returns", on_sales_returns)}
                            </>
                        })}
                        {render_l2_group("采购业务", l2_supply_purchase, html! {
                            <>
                                {render_item("采购订单", "purchase_orders", on_po)}
                                {render_item("采购合同", "purchase_contracts", on_pc)}
                                {render_item("采购收货", "purchase_receipts", on_pr)}
                                {render_item("采购退货", "purchase_returns", on_pret)}
                            </>
                        })}
                        {render_l2_group("价格管理", l2_supply_price, html! {
                            <>
                                {render_item("销售价格", "sales_prices", on_sales_prices)}
                                {render_item("采购价格", "purchase_prices", on_purchase_prices)}
                            </>
                        })}
                    </>
                })}

                {render_l1_group("仓储与质量", l1_inventory, html! {
                    <>
                        {render_l2_group("仓储作业", l2_inv_ops, html! {
                            <>
                                {render_item("库存查询", "inventory", on_inv_stock)}
                                {render_item("库存调拨", "transfers", on_inv_transfer)}
                                {render_item("库存盘点", "counts", on_inv_count)}
                                {render_item("库存调整", "inventory_adjustments", on_inv_adj)}
                            </>
                        })}
                        {render_l2_group("质量管理", l2_inv_qa, html! {
                            <>
                                {render_item("质量检验", "quality_inspection", on_qa_insp)}
                                {render_item("采购检验", "purchase_inspections", on_qa_po)}
                                {render_item("供应商评估", "supplier_evaluation", on_qa_supp)}
                            </>
                        })}
                    </>
                })}

                {render_l1_group("财务核算", l1_finance, html! {
                    <>
                        {render_l2_group("账务管理", l2_fin_acc, html! {
                            <>
                                {render_item("资金管理", "fund_management", on_fin_fund)}
                                {render_item("固定资产", "fixed_assets", on_fin_asset)}
                                {render_item("会计科目", "account_subjects", on_fin_subj)}
                                {render_item("记账凭证", "vouchers", on_fin_voucher)}
                            </>
                        })}
                        {render_l2_group("应收应付", l2_fin_ar_ap, html! {
                            <>
                                {render_item("销售发票", "ar_invoices", on_fin_ar_inv)}
                                {render_item("采购发票", "ap_invoices", on_fin_ap_inv)}
                                {render_item("应付付款申请", "ap_payment_requests", on_fin_ap_req)}
                                {render_item("应付付款", "ap_payments", on_fin_ap_pay)}
                                {render_item("客户信用", "customer_credits", on_fin_credit)}
                            </>
                        })}
                        {render_l2_group("成本与分析", l2_fin_cost, html! {
                            <>
                                {render_item("成本归集", "cost_collections", on_fin_cost)}
                                {render_item("辅助核算", "assist_accounting", on_fin_assist)}
                            </>
                        })}
                    </>
                })}

                {render_l1_group("面料行业特色", l1_fabric, html! {
                    <>
                        {render_l2_group("生产与批次", l2_fab_prod, html! {
                            <>
                                {render_item("批次管理", "batches", on_fab_batch)}
                                {render_item("染缸管理", "dye_batches", on_fab_dye)}
                                {render_item("染料配方", "dye_recipes", on_fab_recipe)}
                                {render_item("坯布管理", "greige_fabrics", on_fab_greige)}
                            </>
                        })}
                        {render_l2_group("特色工具", l2_fab_tools, html! {
                            <>
                                {render_item("双单位转换", "dual_unit_converter", on_fab_dual)}
                                {render_item("五维查询", "five_dimensions", on_fab_five)}
                                {render_item("业务追溯", "business_trace", on_fab_trace)}
                            </>
                        })}
                    </>
                })}

                {render_l1_group("系统与分析", l1_system, html! {
                    <>
                        {render_l2_group("报表中心", l2_sys_report, html! {
                            <>
                                {render_item("销售分析", "sales_analysis", on_sys_sales_rep)}
                                {render_item("财务分析", "financial_analysis", on_sys_fin_rep)}
                            </>
                        })}
                        {render_l2_group("系统管理", l2_sys_admin, html! {
                            {render_item("系统设置", "system_settings", on_sys_settings)}
                        })}
                    </>
                })}
                
            </div>
        </nav>
    }
}
