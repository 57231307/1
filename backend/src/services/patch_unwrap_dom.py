import re
import os

def replace_in_file(filepath):
    with open(filepath, "r") as f:
        content = f.read()

    # Add import
    if "use crate::utils::dom_helper;" not in content:
        content = content.replace("use yew::prelude::*;", "use yew::prelude::*;\nuse crate::utils::dom_helper;")
        content = content.replace("use yew::{html, Component, Context, Html};", "use yew::{html, Component, Context, Html};\nuse crate::utils::dom_helper;")

    # Replace manual get_val with dom_helper functions
    manual_get_val = re.compile(r'let get_val = \|id: &str\| -> String \{.*?\};', re.DOTALL)
    manual_get_opt = re.compile(r'let get_opt = \|id: &str\| -> Option<String> \{.*?\};', re.DOTALL)

    content = manual_get_val.sub('let get_val = |id: &str| -> String { dom_helper::get_input_value(id).or_else(|| dom_helper::get_textarea_value(id)).unwrap_or_default() };', content)
    content = manual_get_opt.sub('let get_opt = |id: &str| -> Option<String> { let v = get_val(id); if v.is_empty() { None } else { Some(v) } };', content)

    with open(filepath, "w") as f:
        f.write(content)

replace_in_file("/home/root0/桌面/121/1/frontend/src/pages/fabric_order.rs")
replace_in_file("/home/root0/桌面/121/1/frontend/src/pages/dye_recipe.rs")
replace_in_file("/home/root0/桌面/121/1/frontend/src/pages/greige_fabric.rs")

# Also fix purchase_inspection.rs which has manual unwraps
with open("/home/root0/桌面/121/1/frontend/src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

if "use crate::utils::dom_helper;" not in content:
    content = content.replace("use yew::prelude::*;", "use yew::prelude::*;\nuse crate::utils::dom_helper;")

content = content.replace(
    'let receipt_id = web_sys::window().unwrap().document().unwrap().get_element_by_id("create-receipt-id").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap_or(0);',
    'let receipt_id = dom_helper::get_numeric_value("create-receipt-id").unwrap_or(0.0) as i32;'
)
content = content.replace(
    'let supplier_id = web_sys::window().unwrap().document().unwrap().get_element_by_id("create-supplier-id").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap_or(0);',
    'let supplier_id = dom_helper::get_numeric_value("create-supplier-id").unwrap_or(0.0) as i32;'
)
content = content.replace(
    'let date = web_sys::window().unwrap().document().unwrap().get_element_by_id("create-date").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value();',
    'let date = dom_helper::get_input_value("create-date").unwrap_or_default();'
)

content = content.replace(
    'let pass_qty = web_sys::window().unwrap().document().unwrap().get_element_by_id("complete-pass-qty").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value();',
    'let pass_qty = dom_helper::get_input_value("complete-pass-qty").unwrap_or_default();'
)
content = content.replace(
    'let reject_qty = web_sys::window().unwrap().document().unwrap().get_element_by_id("complete-reject-qty").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value();',
    'let reject_qty = dom_helper::get_input_value("complete-reject-qty").unwrap_or_default();'
)
content = content.replace(
    'let result = web_sys::window().unwrap().document().unwrap().get_element_by_id("complete-result").unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap().value();',
    'let result = dom_helper::get_select_value("complete-result").unwrap_or_default();'
)

with open("/home/root0/桌面/121/1/frontend/src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)

