use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub document_type: String,
    pub document_id: String,
    #[prop_or_default]
    pub class: String,
}

#[derive(Serialize)]
struct PrintLogRequest {
    pub document_type: String,
    pub document_id: String,
    pub action: String,
}

#[function_component(TrackedPrintButton)]
pub fn tracked_print_button(props: &Props) -> Html {
    let print_count = use_state(|| 0);

    let onclick = {
        let doc_type = props.document_type.clone();
        let doc_id = props.document_id.clone();
        let count = print_count.clone();
        
        Callback::from(move |_| {
            let current_count = *count;
            if current_count > 0 {
                let msg = format!("该单据已被打印过 {} 次，是否继续重复打印？", current_count);
                if let Some(win) = web_sys::window() {
                    if !win.confirm_with_message(&msg).unwrap_or(false) {
                        return;
                    }
                }
            }

            let dt = doc_type.clone();
            let di = doc_id.clone();
            
            spawn_local(async move {
                // Log the print action to the backend
                let req = PrintLogRequest {
                    document_type: dt,
                    document_id: di,
                    action: "PRINT".to_string(),
                };
                let _ = ApiService::post::<(), PrintLogRequest>("/api/v1/erp/operation-logs", &req).await;
            });

            count.set(current_count + 1);
            if let Some(win) = web_sys::window() {
                let _ = win.print();
            }
        })
    };

    let base_class = "btn-outline text-slate-600 border-slate-300 hover:bg-slate-50 flex items-center gap-2 transition-colors";
    let final_class = if props.class.is_empty() { base_class.to_string() } else { format!("{} {}", base_class, props.class) };

    html! {
        <button onclick={onclick} class={final_class} title="带有防重复追踪的安全打印">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"></path></svg>
            {"打印单据"}
            if *print_count > 0 {
                <span class="ml-1 px-1.5 py-0.5 rounded-full bg-yellow-100 text-yellow-800 text-[10px] font-bold">{*print_count}</span>
            }
        </button>
    }
}
