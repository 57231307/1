use crate::app::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NavigationProps {
    pub current_page: String,
}

#[function_component(Navigation)]
pub fn navigation(props: &NavigationProps) -> Html {
    let navigator = use_navigator();

    // 折叠状态
    let dashboard_open = use_state(|| true);
    let system_open = use_state(|| true);
    let basic_data_open = use_state(|| true);
    let sales_open = use_state(|| true);

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

    let on_products = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Products);
            }
        })
    };

    let on_sales_returns = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::SalesReturns);
            }
        })
    };

    html! {
        <nav class="navigation">
            <div class="nav-brand">{"面料 ERP 系统"}</div>
            <div class="nav-menu">
                <div class="nav-group">
                    <div class="nav-group-header" onclick={{
                        let dashboard_open = dashboard_open.clone();
                        Callback::from(move |_| dashboard_open.set(!*dashboard_open))
                    }}>
                        <span class="nav-group-title">{"仪表盘"}</span>
                        <span class={if *dashboard_open { "nav-group-icon open" } else { "nav-group-icon" }}>{"▼"}</span>
                    </div>
                    {if *dashboard_open {
                        html! {
                            <div class="nav-group-content">
                                <a class={if props.current_page == "dashboard" { "nav-item active" } else { "nav-item" }} onclick={on_dashboard}>
                                    {"首页"}
                                </a>
                            </div>
                        }
                    } else { html! {} }}
                </div>

                <div class="nav-group">
                    <div class="nav-group-header" onclick={{
                        let system_open = system_open.clone();
                        Callback::from(move |_| system_open.set(!*system_open))
                    }}>
                        <span class="nav-group-title">{"系统管理"}</span>
                        <span class={if *system_open { "nav-group-icon open" } else { "nav-group-icon" }}>{"▼"}</span>
                    </div>
                    {if *system_open {
                        html! {
                            <div class="nav-group-content">
                                <a class={if props.current_page == "users" { "nav-item active" } else { "nav-item" }} onclick={on_users}>
                                    {"用户管理"}
                                </a>
                            </div>
                        }
                    } else { html! {} }}
                </div>

                <div class="nav-group">
                    <div class="nav-group-header" onclick={{
                        let basic_data_open = basic_data_open.clone();
                        Callback::from(move |_| basic_data_open.set(!*basic_data_open))
                    }}>
                        <span class="nav-group-title">{"基础数据"}</span>
                        <span class={if *basic_data_open { "nav-group-icon open" } else { "nav-group-icon" }}>{"▼"}</span>
                    </div>
                    {if *basic_data_open {
                        html! {
                            <div class="nav-group-content">
                                <a class={if props.current_page == "products" { "nav-item active" } else { "nav-item" }} onclick={on_products}>
                                    {"产品管理"}
                                </a>
                            </div>
                        }
                    } else { html! {} }}
                </div>
            </div>

                <div class="nav-group">
                    <div class="nav-group-header" onclick={{
                        let sales_open = sales_open.clone();
                        Callback::from(move |_| sales_open.set(!*sales_open))
                    }}>
                        <span class="nav-group-title">{"销售管理"}</span>
                        <span class={if *sales_open { "nav-group-icon open" } else { "nav-group-icon" }}>{"▼"}</span>
                    </div>
                    {if *sales_open {
                        html! {
                            <div class="nav-group-content">
                                <a class={if props.current_page == "sales_returns" { "nav-item active" } else { "nav-item" }} onclick={on_sales_returns}>
                                    {"销售退货"}
                                </a>
                            </div>
                        }
                    } else { html! {} }}
                </div>
        </nav>
    }
}
