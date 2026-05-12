use yew::prelude::*;
use yew_router::prelude::*;
use crate::route::Route;

#[derive(Properties, PartialEq)]
pub struct NavigationProps {
    pub collapsed: bool,
    pub on_toggle: Callback<()>,
}

#[function_component(Navigation)]
pub fn navigation(props: &NavigationProps) -> Html {
    let collapsed = props.collapsed;
    let on_toggle = props.on_toggle.clone();

    let toggle_nav = Callback::from(move |_| {
        on_toggle.emit(());
    });

    html! {
        <nav class={classes!("sidebar-navigation", collapsed.then_some("collapsed"))}>
            <div class="nav-brand">
                if !collapsed {
                    <span class="brand-name">{"秉羲面料"}</span>
                }
                <button class="nav-toggle" onclick={toggle_nav}>
                    if collapsed {
                        <i class="fas fa-chevron-right"></i>
                    } else {
                        <i class="fas fa-chevron-left"></i>
                    }
                </button>
            </div>

            <div class="nav-menu">
                <NavItem route={Route::Dashboard} icon={"fas fa-tachometer-alt"} label={"仪表盘"} {collapsed} />
                <NavItem route={Route::Fabrics} icon={"fas fa-layer-group"} label={"面料管理"} {collapsed} />
                <NavItem route={Route::Inventory} icon={"fas fa-warehouse"} label={"库存管理"} {collapsed} />
                <NavItem route={Route::Orders} icon={"fas fa-shopping-cart"} label={"订单管理"} {collapsed} />
                <NavItem route={Route::Suppliers} icon={"fas fa-truck"} label={"供应商"} {collapsed} />
                <NavItem route={Route::Reports} icon={"fas fa-chart-bar"} label={"报表分析"} {collapsed} />
                <NavItem route={Route::Settings} icon={"fas fa-cog"} label={"系统设置"} {collapsed} />
            </div>

            <div class="nav-footer">
                <NavItem route={Route::Profile} icon={"fas fa-user"} label={"个人中心"} {collapsed} />
            </div>
        </nav>
    }
}

#[derive(Properties, PartialEq)]
struct NavItemProps {
    pub route: Route,
    pub icon: &'static str,
    pub label: &'static str,
    pub collapsed: bool,
}

#[function_component(NavItem)]
fn nav_item(props: &NavItemProps) -> Html {
    let navigator = use_navigator().unwrap();
    let route = props.route.clone();
    let onclick = Callback::from(move |_| {
        navigator.push(&route);
    });

    html! {
        <div class="nav-item" {onclick}>
            <i class={props.icon}></i>
            if !props.collapsed {
                <span class="nav-label">{ props.label }</span>
            }
        </div>
    }
}
