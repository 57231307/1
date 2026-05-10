use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LoadingStateProps {
    pub message: String,
}

#[function_component(LoadingState)]
pub fn loading_state(props: &LoadingStateProps) -> Html {
    html! {
        <div class="loading-container">
            <div class="spinner-large"></div>
            <p>{&props.message}</p>
        </div>
    }
}
