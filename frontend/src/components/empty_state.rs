use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct EmptyStateProps {
    pub icon: String,
    pub title: String,
    pub description: String,
}

#[function_component(EmptyState)]
pub fn empty_state(props: &EmptyStateProps) -> Html {
    html! {
        <div class="empty-state">
            <div class="empty-icon">{&props.icon}</div>
            <h3>{&props.title}</h3>
            <p>{&props.description}</p>
        </div>
    }
}
