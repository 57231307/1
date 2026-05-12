use yew::prelude::*;

#[function_component(Header)]
pub fn header() -> Html {
    html! {
        <header class="app-header">
            <div class="header-content">
                <h1 class="header-title">{"秉羲面料管理系统"}</h1>
            </div>
        </header>
    }
}
