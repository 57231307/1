import os

def fix_crud_service_import(file_path):
    with open(file_path, 'r') as f:
        content = f.read()
    
    if "use crate::services::crud_service::CrudService;" not in content:
        # Find the first use crate::services::... and append CrudService
        lines = content.split('\n')
        for i, line in enumerate(lines):
            if "use crate::services::" in line:
                lines.insert(i+1, "use crate::services::crud_service::CrudService;")
                break
        
        with open(file_path, 'w') as f:
            f.write('\n'.join(lines))

def fix_contract_pages():
    for page in ['src/pages/sales_contract.rs', 'src/pages/purchase_contract.rs']:
        path = os.path.join('/home/root0/桌面/121/1/frontend', page)
        with open(path, 'r') as f:
            content = f.read()
        
        content = content.replace("Msg::SearchKeyword(input.value())", "Some(Msg::SearchKeyword(input.value()))")
        
        with open(path, 'w') as f:
            f.write(content)

def fix_account_subject():
    path = '/home/root0/桌面/121/1/frontend/src/pages/account_subject.rs'
    with open(path, 'r') as f:
        content = f.read()
    
    # We need to extract `node.id` to `id` before the html! macro or inside it.
    # The macro uses `on_delete.reform(move |_| node.id)`
    # We can replace it with:
    # let id = node.id;
    # html! { ... onclick={on_delete.reform(move |_| id)} ... }
    # Let's just do a regex replace
    import re
    # We'll just replace `on_delete.reform(move |_| node.id)` with `{ let id = node.id; on_delete.reform(move |_| id) }`
    content = content.replace("on_delete.reform(move |_| node.id)", "{ let id = node.id; on_delete.reform(move |_| id) }")
    
    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    pages = [
        'src/pages/cost_collection.rs',
        'src/pages/finance_payment.rs',
        'src/pages/sales_contract.rs',
        'src/pages/purchase_contract.rs',
        'src/pages/purchase_inspection.rs',
        'src/pages/dye_batch.rs',
        'src/pages/dye_recipe.rs',
        'src/pages/greige_fabric.rs',
    ]
    for page in pages:
        fix_crud_service_import(os.path.join('/home/root0/桌面/121/1/frontend', page))
    
    fix_contract_pages()
    fix_account_subject()
    print("Fixes applied.")
