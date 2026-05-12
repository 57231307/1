use yew::prelude::*;
use crate::components::navigation::Navigation;

#[derive(Properties, PartialEq)]
pub struct MainLayoutProps {
    pub current_page: String,
    pub children: Children,
}

#[function_component(MainLayout)]
pub fn main_layout(props: &MainLayoutProps) -> Html {
    let collapsed = use_state(|| false);

    let toggle = {
        let collapsed = collapsed.clone();
        Callback::from(move |_| collapsed.set(!*collapsed))
    };

    use_effect_with((), |_| {
        web_sys::console::log_1(&"MainLayout rendered!".into());
        || ()
    });

    let nav_class = if *collapsed { "navigation collapsed" } else { "navigation" };
    let main_class = if *collapsed { "main-content expanded" } else { "main-content" };

    html! {
        <div class="app-container">
            <Navigation current_page={props.current_page.clone()} collapsed={*collapsed} on_toggle={toggle} />
            <main class={main_class}>
                {props.children.clone()}
            </main>
        </div>
    }
}
