use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ErrorBoundaryProps {
    pub children: Children,
    #[prop_or_default]
    pub fallback: Option<Html>,
}

#[function_component(ErrorBoundary)]
pub fn error_boundary(props: &ErrorBoundaryProps) -> Html {
    let error = use_state(|| None::<String>);

    let on_retry = {
        let error = error.clone();
        Callback::from(move |_| error.set(None))
    };

    if let Some(err) = (*error).as_ref() {
        return props.fallback.clone().unwrap_or_else(|| html! {
            <div class="error-boundary" style="padding: 40px; text-align: center;">
                <h2 style="color: #ef4444;">{"页面加载出错"}</h2>
                <p style="color: #64748b; margin: 16px 0;">{err}</p>
                <button
                    onclick={on_retry}
                    style="padding: 8px 24px; background: #2563eb; color: white; border: none; border-radius: 4px; cursor: pointer;"
                >
                    {"重试"}
                </button>
            </div>
        });
    }

    html! { { props.children.clone() } }
}
