use crate::app::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NavigationProps {
    pub current_page: String,
}

struct MenuCategory {
    title: &'static str,
    icon: &'static str,
    items: Vec<MenuItem>,
}

struct MenuItem {
    title: &'static str,
    route: Route,
}

#[function_component(Navigation)]
pub fn navigation(props: &NavigationProps) -> Html {
    let navigator = use_navigator();

    let categories = vec![
        MenuCategory {
            title: "工作台",
            icon: "📊",
            items: vec![
                MenuItem { title: "仪表板", route: Route::Dashboard },
            ]
        },
        MenuCategory {
            title: "基础数据",
            icon: "📋",
            items: vec![
                MenuItem { title: "产品管理", route: Route::Products },
                MenuItem { title: "产品分类", route: Route::ProductCategories },
                MenuItem { title: "仓库管理", route: Route::Warehouses },
                MenuItem { title: "部门管理", route: Route::Departments },
                MenuItem { title: "角色管理", route: Route::Roles },
                MenuItem { title: "用户管理", route: Route::Users },
                MenuItem { title: "供应商", route: Route::Suppliers },
                MenuItem { title: "客户管理", route: Route::Customers },
            ]
        },
        MenuCategory {
            title: "销售与CRM",
            icon: "🤝",
            items: vec![
                MenuItem { title: "销售订单", route: Route::Sales },
                MenuItem { title: "面料订单", route: Route::FabricOrders },
                MenuItem { title: "销售合同", route: Route::SalesContracts },
                MenuItem { title: "销售退货", route: Route::SalesReturns },
                MenuItem { title: "销售价格", route: Route::SalesPrices },
                MenuItem { title: "客户信用", route: Route::CustomerCredits },
                MenuItem { title: "销售分析", route: Route::SalesAnalysis },
                MenuItem { title: "CRM线索", route: Route::CrmLeads },
                MenuItem { title: "CRM商机", route: Route::CrmOpportunities },
            ]
        },
        MenuCategory {
            title: "采购管理",
            icon: "🛒",
            items: vec![
                MenuItem { title: "采购订单", route: Route::PurchaseOrders },
                MenuItem { title: "采购合同", route: Route::PurchaseContracts },
                MenuItem { title: "采购入库", route: Route::PurchaseReceipts },
                MenuItem { title: "采购退货", route: Route::PurchaseReturns },
                MenuItem { title: "采购价格", route: Route::PurchasePrices },
                MenuItem { title: "供应商评估", route: Route::SupplierEvaluation },
            ]
        },
        MenuCategory {
            title: "库存管理",
            icon: "📦",
            items: vec![
                MenuItem { title: "当前库存", route: Route::Inventory },
                MenuItem { title: "库存调拨", route: Route::Transfers },
                MenuItem { title: "库存盘点", route: Route::Counts },
                MenuItem { title: "库存调整", route: Route::InventoryAdjustments },
                MenuItem { title: "批次管理", route: Route::Batches },
                MenuItem { title: "业务追溯", route: Route::BusinessTrace },
            ]
        },
        MenuCategory {
            title: "生产与质检",
            icon: "🏭",
            items: vec![
                MenuItem { title: "胚布管理", route: Route::GreigeFabrics },
                MenuItem { title: "染色配方", route: Route::DyeRecipes },
                MenuItem { title: "染色批次", route: Route::DyeBatches },
                MenuItem { title: "采购检验", route: Route::PurchaseInspections },
                MenuItem { title: "质检记录", route: Route::QualityInspection },
            ]
        },
        MenuCategory {
            title: "财务核算",
            icon: "💰",
            items: vec![
                MenuItem { title: "会计科目", route: Route::AccountSubjects },
                MenuItem { title: "财务凭证", route: Route::Vouchers },
                MenuItem { title: "辅助核算", route: Route::AssistAccounting },
                MenuItem { title: "资金管理", route: Route::FundManagement },
                MenuItem { title: "固定资产", route: Route::FixedAssets },
                MenuItem { title: "成本归集", route: Route::CostCollections },
                MenuItem { title: "五维数据", route: Route::FiveDimensions },
                MenuItem { title: "财务分析", route: Route::FinancialAnalysis },
            ]
        },
        MenuCategory {
            title: "应收应付",
            icon: "🧾",
            items: vec![
                MenuItem { title: "应付发票", route: Route::ApInvoices },
                MenuItem { title: "付款申请", route: Route::ApPaymentRequests },
                MenuItem { title: "应付付款", route: Route::ApPayments },
                MenuItem { title: "应付核销", route: Route::ApVerifications },
                MenuItem { title: "应付对账", route: Route::ApReconciliations },
                MenuItem { title: "应收发票", route: Route::ArInvoices },
                MenuItem { title: "发票管理", route: Route::FinanceInvoices },
                MenuItem { title: "收付款", route: Route::FinancePayments },
            ]
        },
    ];

    html! {
        <nav class="w-64 bg-white border-r border-gray-200 h-screen overflow-y-auto flex flex-col">
            <div class="p-4 border-b border-gray-200">
                <h2 class="text-xl font-bold text-gray-800">{"Bingxi ERP"}</h2>
            </div>
            
            <div class="flex-1 py-4">
                {
                    for categories.iter().map(|cat| {
                        html! {
                            <div class="mb-4">
                                <div class="px-4 py-2 flex items-center justify-between text-gray-500 hover:text-gray-700 cursor-pointer transition-colors duration-150">
                                    <div class="flex items-center">
                                        <span class="mr-2 text-lg">{cat.icon}</span>
                                        <span class="font-medium text-sm">{cat.title}</span>
                                    </div>
                                </div>
                                <div class="mt-1">
                                    {
                                        for cat.items.iter().map(|item| {
                                            let nav = navigator.clone();
                                            let route = item.route.clone();
                                            let is_active = props.current_page == item.title;
                                            
                                            let active_class = if is_active {
                                                "bg-blue-50 text-blue-600 border-r-4 border-blue-600 font-medium"
                                            } else {
                                                "text-gray-600 hover:bg-gray-50 hover:text-gray-900 border-r-4 border-transparent"
                                            };
                                            
                                            let onclick = Callback::from(move |_| {
                                                if let Some(n) = &nav {
                                                    n.push(&route);
                                                }
                                            });

                                            html! {
                                                <div 
                                                    class={format!("pl-10 pr-4 py-2 cursor-pointer transition-colors duration-150 text-sm {}", active_class)}
                                                    onclick={onclick}
                                                >
                                                    {item.title}
                                                </div>
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
