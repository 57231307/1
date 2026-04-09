use crate::app::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NavigationProps {
    pub current_page: String,
}

struct MenuGroup {
    title: &'static str,
    icon: &'static str,
    items: Vec<MenuItem>,
}

struct MenuItem {
    name: &'static str,
    route: Route,
}

#[function_component(Navigation)]
pub fn navigation(props: &NavigationProps) -> Html {
    let open_group = use_state(|| String::from(""));
    
    let toggle_group = {
        let open_group = open_group.clone();
        Callback::from(move |group_name: String| {
            if *open_group == group_name {
                open_group.set(String::from("")); // Close if already open
            } else {
                open_group.set(group_name); // Open new group
            }
        })
    };

    let menu_groups = vec![
        MenuGroup {
            title: "工作台", icon: "📊",
            items: vec![MenuItem { name: "仪表板", route: Route::Dashboard }],
        },
        MenuGroup {
            title: "基础数据", icon: "📁",
            items: vec![
                MenuItem { name: "产品管理", route: Route::Products },
                MenuItem { name: "产品分类", route: Route::ProductCategories },
                MenuItem { name: "仓库管理", route: Route::Warehouses },
                MenuItem { name: "供应商", route: Route::Suppliers },
                MenuItem { name: "客户管理", route: Route::Customers },
                MenuItem { name: "双单位换算", route: Route::DualUnitConverter },
            ],
        },
        MenuGroup {
            title: "销售与CRM", icon: "🤝",
            items: vec![
                MenuItem { name: "销售订单", route: Route::Sales },
                MenuItem { name: "销售发货", route: Route::SalesDeliveries },
                MenuItem { name: "面料订单", route: Route::FabricOrders },
                MenuItem { name: "销售合同", route: Route::SalesContracts },
                MenuItem { name: "销售退货", route: Route::SalesReturns },
                MenuItem { name: "销售价格", route: Route::SalesPrices },
                MenuItem { name: "客户对账单", route: Route::CustomerStatement },
                MenuItem { name: "客户信用", route: Route::CustomerCredits },
                MenuItem { name: "销售分析", route: Route::SalesAnalysis },
                MenuItem { name: "CRM线索", route: Route::CrmLeads },
                MenuItem { name: "CRM商机", route: Route::CrmOpportunities },
            ],
        },
        MenuGroup {
            title: "库存管理", icon: "📦",
            items: vec![
                MenuItem { name: "库存查询", route: Route::Inventory },
                MenuItem { name: "库存预留", route: Route::InventoryReservations },
                MenuItem { name: "库存盘点", route: Route::Counts },
                MenuItem { name: "库存调拨", route: Route::Transfers },
                MenuItem { name: "库存调整", route: Route::InventoryAdjustments },
            ],
        },
        MenuGroup {
            title: "采购管理", icon: "🛒",
            items: vec![
                MenuItem { name: "采购订单", route: Route::PurchaseOrders },
                MenuItem { name: "采购合同", route: Route::PurchaseContracts },
                MenuItem { name: "采购收货", route: Route::PurchaseReceipts },
                MenuItem { name: "采购退货", route: Route::PurchaseReturns },
                MenuItem { name: "采购价格", route: Route::PurchasePrices },
                MenuItem { name: "入厂检验", route: Route::PurchaseInspections },
                MenuItem { name: "供应商评价", route: Route::SupplierEvaluation },
            ],
        },
        MenuGroup {
            title: "财务与成本", icon: "💰",
            items: vec![
                MenuItem { name: "应收账款", route: Route::ArInvoices },
                MenuItem { name: "应付账款", route: Route::ApInvoices },
                MenuItem { name: "收款核销", route: Route::ArVerifications },
                MenuItem { name: "付款核销", route: Route::ApVerifications },
                MenuItem { name: "资金管理", route: Route::FundManagement },
                MenuItem { name: "财务分析", route: Route::FinancialAnalysis },
                MenuItem { name: "凭证管理", route: Route::Vouchers },
                MenuItem { name: "预算管理", route: Route::BudgetManagement },
                MenuItem { name: "成本收集", route: Route::CostCollections },
                MenuItem { name: "辅助核算", route: Route::AssistAccounting },
                MenuItem { name: "业务追溯", route: Route::BusinessTrace },
                MenuItem { name: "固定资产", route: Route::FixedAssets },
            ],
        },
        MenuGroup {
            title: "系统设置", icon: "⚙️",
            items: vec![
                MenuItem { name: "用户管理", route: Route::Users },
                MenuItem { name: "角色管理", route: Route::Roles },
                MenuItem { name: "部门管理", route: Route::Departments },
                MenuItem { name: "操作日志", route: Route::OperationLogs },
            ],
        },
    ];

    html! {
        <nav class="sidebar">
            <div class="sidebar-header">
                <div class="logo">{"Bingxi ERP"}</div>
            </div>
            <div class="sidebar-menu">
                {
                    for menu_groups.into_iter().map(|group| {
                        let group_title = group.title.to_string();
                        let is_open = *open_group == group_title;
                        let icon_class = if is_open { "nav-group-icon open" } else { "nav-group-icon" };
                        let sub_items_class = if is_open { "nav-sub-items open" } else { "nav-sub-items" };
                        
                        let on_header_click = {
                            let toggle_group = toggle_group.clone();
                            let group_title_clone = group_title.clone();
                            Callback::from(move |_| toggle_group.emit(group_title_clone.clone()))
                        };

                        html! {
                            <div class="nav-group">
                                <div class="nav-group-header" onclick={on_header_click}>
                                    <span>
                                        <span class="icon">{group.icon}</span>
                                        {group.title}
                                    </span>
                                    <span class={icon_class}>{"▶"}</span>
                                </div>
                                <div class={sub_items_class}>
                                    {
                                        for group.items.into_iter().map(|item| {
                                            let is_active = props.current_page == item.name;
                                            let class = if is_active { "nav-item active" } else { "nav-item" };
                                            html! {
                                                <Link<Route> to={item.route} classes={class}>
                                                    {item.name}
                                                </Link<Route>>
                                            }
                                        })
                                    }
                                </div>
                            </div>
                        }
                    })
                }
            </div>
        </nav>
    }
}
