import re
import os

# 1. Fix Storage::set_item
with open("/home/root0/桌面/121/1/frontend/src/utils/storage.rs", "r") as f:
    storage_content = f.read()

if "pub fn set_item(key: &str, value: &str)" not in storage_content:
    set_item_fn = """
    pub fn set_item(key: &str, value: &str) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(key, value);
            }
        }
    }
"""
    storage_content = storage_content.replace("pub struct Storage;", "pub struct Storage;\n" + set_item_fn)
    
    # Also add get_item if missing
    if "pub fn get_item(key: &str) -> Option<String>" not in storage_content:
        get_item_fn = """
    pub fn get_item(key: &str) -> Option<String> {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                return storage.get_item(key).ok().flatten();
            }
        }
        None
    }
"""
        storage_content = storage_content.replace("pub struct Storage;\n", "pub struct Storage;\n" + get_item_fn)

    with open("/home/root0/桌面/121/1/frontend/src/utils/storage.rs", "w") as f:
        f.write(storage_content)

# 2. Fix duplicated rendered and missing Msg
def fix_page(filepath, item_type, msg_enum_start):
    with open(filepath, "r") as f:
        content = f.read()
        
    # merge duplicated rendered
    # We find two rendered methods
    # fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
    #     if first_render {
    #         ctx.link().send_message(Msg::LoadInvoices);
    #     }
    # }
    # fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
    #     if self.print_trigger {
    
    content = re.sub(r'fn rendered\(&mut self, ctx: &Context<Self>, first_render: bool\) \{\s*if first_render \{\s*ctx\.link\(\)\.send_message\((.*?)\);\s*\}\s*\}', r'', content)
    
    # modify the remaining rendered
    content = re.sub(r'fn rendered\(&mut self, ctx: &Context<Self>, _first_render: bool\) \{', r'fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {\n        if first_render {\n            ctx.link().send_message(\1);\n        }', content)
    
    # check if Msg::Print... is missing from the enum definition
    # Some pages had Msg::PrintInvoice but it was inserted wrong?
    # Let's find `pub enum Msg {` and check if PrintInvoice is there.
    # Ah, the problem was my sed script added to `Msg::Refresh => {` but it didn't add it to the Enum! 
    # Wait, `content.replace("    Refresh,\n    CloseModal,", "    Refresh,\n    PrintInvoice(ArInvoice),\n    ClearPrint,\n    CloseModal,")` didn't work because they might not be consecutive.
    
    if f"Print{item_type}" not in content.split("pub enum Msg {")[1].split("}")[0]:
        content = content.replace("    CloseModal,", f"    CloseModal,\n    Print{item_type}({item_type}),\n    ClearPrint,")
        
    with open(filepath, "w") as f:
        f.write(content)

fix_page("/home/root0/桌面/121/1/frontend/src/pages/ar_invoice.rs", "Invoice", "ArInvoice")
fix_page("/home/root0/桌面/121/1/frontend/src/pages/ap_invoice.rs", "Invoice", "ApInvoice")
fix_page("/home/root0/桌面/121/1/frontend/src/pages/sales_return.rs", "Return", "SalesReturn")

