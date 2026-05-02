import re
import os

# 1. Fix sales_contract.rs and purchase_contract.rs component issue
def fix_cancel_modal(filepath):
    with open(filepath, "r") as f:
        content = f.read()
    content = content.replace("impl Component for CancelContractModalProps {", "impl Component for CancelContractModal {")
    content = content.replace("<CancelContractModalProps", "<CancelContractModal")
    
    # Ensure CancelContractModal struct exists
    if "pub struct CancelContractModal {" not in content:
        # replace CancelContractModalProps { ... } with both
        pattern = r"pub struct CancelContractModalProps \{.*?\}"
        # actually let's just append it before impl Component
        impl_idx = content.find("impl Component for CancelContractModal {")
        if impl_idx != -1:
            content = content[:impl_idx] + "pub struct CancelContractModal { pub reason: String }\n\n" + content[impl_idx:]
    with open(filepath, "w") as f:
        f.write(content)

fix_cancel_modal("src/pages/sales_contract.rs")
fix_cancel_modal("src/pages/purchase_contract.rs")

# 2. Fix sales_order.rs status field
with open("src/pages/sales_order.rs", "r") as f:
    content = f.read()
    # Looking at the struct, it doesn't have status or status_filter or filter_status. Let's add it.
    if "filter_status: String," not in content:
        content = content.replace("page_size: u64,", "page_size: u64,\n    filter_status: String,")
    
    # update create fn
    if "filter_status: String::from(\"全部\")," not in content:
        content = content.replace("page_size: 20,", "page_size: 20,\n            filter_status: String::from(\"全部\"),")
    
    # fix the use site
    content = content.replace("self.status.as_deref() ==", "self.filter_status ==")
    content = content.replace("self.status.clone()", "self.filter_status.clone()")
    content = content.replace("self.status ==", "self.filter_status ==")
with open("src/pages/sales_order.rs", "w") as f:
    f.write(content)

# 3. Fix purchase_contract.rs CreatePurchaseContractRequest vs ExecutePurchaseContractRequest
with open("src/pages/purchase_contract.rs", "r") as f:
    content = f.read()
    content = content.replace("crate::models::purchase_contract::CreatePurchaseContractRequest {", "crate::models::purchase_contract::ExecutePurchaseContractRequest {")
with open("src/pages/purchase_contract.rs", "w") as f:
    f.write(content)

# 4. Fix inventory_stock.rs lifetime issue
with open("src/pages/inventory_stock.rs", "r") as f:
    content = f.read()
    # The issue is `b` and `c` are references that escape. We need to clone them outside.
    # The code looks like `match InventoryService::list_stock_fabric(1, 50, b, c).await {`
    # Let's find how `b` and `c` are created.
    # We should do:
    # let b = b.clone();
    # let c = c.clone();
    # before `spawn_local(async move {`
    
    match = re.search(r'(let b_loading = loading\.clone\(\);\n\s*wasm_bindgen_futures::spawn_local\(async move \{)', content)
    if match:
        content = content.replace(match.group(1), "let b_loading = loading.clone();\n            let b_owned = b.map(|s| s.to_string());\n            let c_owned = c.map(|s| s.to_string());\n            wasm_bindgen_futures::spawn_local(async move {")
        content = content.replace("list_stock_fabric(1, 50, b, c)", "list_stock_fabric(1, 50, b_owned.as_deref(), c_owned.as_deref())")
        content = content.replace("if b.is_none() && c.is_none()", "if b_owned.is_none() && c_owned.is_none()")
        
with open("src/pages/inventory_stock.rs", "w") as f:
    f.write(content)

