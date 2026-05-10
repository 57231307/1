use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ConfirmDialogProps {
    pub title: String,
    pub message: String,
    pub confirm_text: String,
    pub cancel_text: String,
    pub confirm_class: String,
    pub on_confirm: Callback<()>,
    pub on_cancel: Callback<()>,
    pub visible: bool,
}

#[function_component(ConfirmDialog)]
pub fn confirm_dialog(props: &ConfirmDialogProps) -> Html {
    if !props.visible {
        return html! {};
    }

    let on_confirm = props.on_confirm.clone();
    let on_cancel = props.on_cancel.clone();
    let on_cancel2 = props.on_cancel.clone();
    let on_cancel3 = props.on_cancel.clone();

    let confirm_class = props.confirm_class.clone();

    html! {
        <div class="modal-overlay" onclick={Callback::from(move |_| on_cancel.emit(()))}>
            <div class="modal-content confirm-dialog" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                <div class="modal-header">
                    <h3>{&props.title}</h3>
                    <button class="close-btn" onclick={Callback::from(move |_| on_cancel2.emit(()))}>{"×"}</button>
                </div>
                <div class="modal-body">
                    <p>{&props.message}</p>
                </div>
                <div class="modal-footer">
                    <button class="btn btn-secondary" onclick={Callback::from(move |_| on_cancel3.emit(()))}>
                        {&props.cancel_text}
                    </button>
                    <button class={format!("btn {}", confirm_class)} onclick={Callback::from(move |_| on_confirm.emit(()))}>
                        {&props.confirm_text}
                    </button>
                </div>
            </div>
        </div>
    }
}
