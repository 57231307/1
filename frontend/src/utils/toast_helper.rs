use wasm_bindgen::JsCast;
use web_sys::{window, Element, HtmlElement};

pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

pub fn show_toast(message: &str, toast_type: ToastType) {
    let window = match window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let container_id = "global-toast-container";
    let container = match document.get_element_by_id(container_id) {
        Some(el) => el,
        None => {
            let body = match document.body() {
                Some(b) => b,
                None => return,
            };
            let new_container = document.create_element("div").unwrap();
            new_container.set_id(container_id);
            new_container.set_class_name("fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none");
            body.append_child(&new_container).unwrap();
            new_container
        }
    };

    let toast = document.create_element("div").unwrap();
    let bg_color = match toast_type {
        ToastType::Success => "bg-green-500",
        ToastType::Error => "bg-red-500",
        ToastType::Warning => "bg-yellow-500",
        ToastType::Info => "bg-blue-500",
    };

    toast.set_class_name(&format!("{} text-white px-4 py-3 rounded shadow-lg transition-opacity duration-300 pointer-events-auto", bg_color));
    toast.set_inner_html(message);

    container.append_child(&toast).unwrap();

    let toast_clone = toast.clone();
    let bg_color_clone = bg_color.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        gloo::timers::future::TimeoutFuture::new(3000).await;
        toast_clone.set_class_name(&format!("{} text-white px-4 py-3 rounded shadow-lg transition-opacity duration-300 pointer-events-auto opacity-0", bg_color_clone));
        let t2 = toast_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            gloo::timers::future::TimeoutFuture::new(300).await;
            if let Some(parent) = t2.parent_node() {
                let _ = parent.remove_child(&t2);
            }
        });
    });
}

pub fn show_success(message: &str) {
    show_toast(message, ToastType::Success);
}

pub fn show_error(message: &str) {
    show_toast(message, ToastType::Error);
}
