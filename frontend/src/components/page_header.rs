use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct PageHeaderProps {
    pub title: String,
    pub subtitle: Option<String>,
    pub children: Children,
}

#[function_component(PageHeader)]
pub fn page_header(props: &PageHeaderProps) -> Html {
    html! {
        <div class="page-header">
            <div class="page-header-left">
                <h1>{&props.title}</h1>
                if let Some(subtitle) = &props.subtitle {
                    <p class="page-subtitle">{subtitle}</p>
                }
            </div>
            <div class="page-header-right">
                {props.children.clone()}
            </div>
        </div>
    }
}
