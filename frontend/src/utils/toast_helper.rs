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

    // 查找或创建 toast 容器
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
            // fixed top-4 right-4 z-50 flex flex-col gap-2
            new_container.set_class_name("fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none");
            body.append_child(&new_container).unwrap();
            new_container
        }
    };

    // 创建 toast 元素
    let toast = document.create_element("div").unwrap();
    let bg_color = match toast_type {
        ToastType::Success => "bg-green-500",
        ToastType::Error => "bg-red-500",
        ToastType::Warning => "bg-yellow-500",
        ToastType::Info => "bg-blue-500",
    };
    
    // animate-fade-in-down is an assumption of tailwind config, we'll use inline styles or existing tailwind classes
    toast.set_class_name(&format!("{} text-white px-4 py-3 rounded shadow-lg transition-opacity duration-300 pointer-events-auto", bg_color));
    toast.set_inner_html(message);

    container.append_child(&toast).unwrap();

    // 3秒后移除
    let toast_clone = toast.clone();
    gloo_timers::callback::Timeout::new(3000, move || {
        toast_clone.set_class_name(&format!("{} text-white px-4 py-3 rounded shadow-lg transition-opacity duration-300 pointer-events-auto opacity-0", bg_color));
        let t2 = toast_clone.clone();
        gloo_timers::callback::Timeout::new(300, move || {
            if let Some(parent) = t2.parent_node() {
                parent.remove_child(&t2).unwrap();
            }
        }).forget();
    }).forget();
}

pub fn show_success(message: &str) {
    show_toast(message, ToastType::Success);
}

pub fn show_error(message: &str) {
    show_toast(message, ToastType::Error);
}

/// 替代 gloo_dialogs::confirm
/// 因为 confirm 需要阻塞并返回 bool，WASM 中无法用纯 DOM 做到阻塞返回。
/// 如果要彻底替换，需要重构业务逻辑为回调。
/// 暂时我们保留 confirm，只替换 alert。
pub fn confirm(message: &str) -> bool {
    gloo_dialogs::confirm(message)
}
