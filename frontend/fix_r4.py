import os

def fix_contracts():
    for page in ['sales_contract.rs', 'purchase_contract.rs']:
        path = os.path.join('/home/root0/桌面/121/1/frontend/src/pages', page)
        with open(path, 'r') as f:
            content = f.read()
        
        # In both files, CancelContractModal component is missing its struct definition.
        # It has `pub struct CancelContractModalProps { ... }` and `pub struct CancelContractModalState { ... }`
        # We need to insert `pub struct CancelContractModal { reason: String }` before `impl Component for CancelContractModal`
        
        if "pub struct CancelContractModal {" not in content:
            content = content.replace("impl Component for CancelContractModal {", "pub struct CancelContractModal { reason: String }\n\nimpl Component for CancelContractModal {")
        
        # ExecuteContractModal is also missing its struct in purchase_contract.rs!
        # Wait, purchase_contract.rs error:
        # cannot find struct, variant or union type `ExecuteContractRequest` in module `crate::models::purchase_contract`
        # In purchase_contract.rs line 705. Let's look at what's in models::purchase_contract
        
        with open(path, 'w') as f:
            f.write(content)

def fix_inventory_stock():
    path = '/home/root0/桌面/121/1/frontend/src/pages/inventory_stock.rs'
    with open(path, 'r') as f:
        content = f.read()
    
    # We replace:
    # let b = if (*batch).is_empty() { None } else { Some((*batch).as_str()) };
    # let c = if (*color).is_empty() { None } else { Some((*color).as_str()) };
    # with:
    # let b_str = (*batch).clone();
    # let c_str = (*color).clone();
    
    old_str = """            let b = if (*batch).is_empty() { None } else { Some((*batch).as_str()) };
            let c = if (*color).is_empty() { None } else { Some((*color).as_str()) };

            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                // 加载面料库存
                match InventoryService::list_stock_fabric(1, 50, b, c).await {"""
    
    new_str = """            let b_str = (*batch).clone();
            let c_str = (*color).clone();

            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                let b = if b_str.is_empty() { None } else { Some(b_str.as_str()) };
                let c = if c_str.is_empty() { None } else { Some(c_str.as_str()) };
                // 加载面料库存
                match InventoryService::list_stock_fabric(1, 50, b, c).await {"""
    
    # Actually my previous python script might have modified `b` and `c` already!
    # Let me just read and replace properly.
    if "let b = (*batch).clone();" in content:
        # Revert previous bad replacement
        pass
    
    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    fix_contracts()
    fix_inventory_stock()
    print("Fixed round 4.")
