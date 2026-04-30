use yew::prelude::*;
use crate::components::navigation::Navigation;

#[derive(Properties, PartialEq)]
pub struct MainLayoutProps {
    pub current_page: String,
    pub children: Children,
}

#[function_component(MainLayout)]
pub fn main_layout(props: &MainLayoutProps) -> Html {
    html! {
        <div class="app-container">
            <Navigation current_page={props.current_page.clone()} />
            <main class="main-content">
                {props.children.clone()}
            </main>
        </div>
    }
}
