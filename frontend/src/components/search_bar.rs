use yew::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[derive(Properties, PartialEq, Clone)]
pub struct SearchBarProps {
    pub placeholder: String,
    pub on_search: Callback<String>,
    pub on_reset: Callback<()>,
}

#[function_component(SearchBar)]
pub fn search_bar(props: &SearchBarProps) -> Html {
    let search_value = use_state(|| String::new());

    let on_input = {
        let search_value = search_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                search_value.set(input.value());
            }
        })
    };

    let on_search_click = {
        let search_value = search_value.clone();
        let on_search = props.on_search.clone();
        Callback::from(move |_e: MouseEvent| {
            on_search.emit((*search_value).clone());
        })
    };

    let on_key_press = {
        let search_value = search_value.clone();
        let on_search = props.on_search.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                on_search.emit((*search_value).clone());
            }
        })
    };

    let on_reset_click = {
        let search_value = search_value.clone();
        let on_reset = props.on_reset.clone();
        Callback::from(move |_e: MouseEvent| {
            search_value.set(String::new());
            on_reset.emit(());
        })
    };

    html! {
        <div class="search-bar">
            <div class="search-input-wrapper">
                <span class="search-icon">{"🔍"}</span>
                <input
                    type="text"
                    class="search-input"
                    placeholder={props.placeholder.clone()}
                    value={(*search_value).clone()}
                    oninput={on_input}
                    onkeypress={on_key_press}
                />
            </div>
            <button class="btn btn-primary" onclick={on_search_click}>
                {"搜索"}
            </button>
            <button class="btn btn-secondary" onclick={on_reset_click}>
                {"重置"}
            </button>
        </div>
    }
}
