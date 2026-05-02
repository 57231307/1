import os

def fix_responses():
    for page in ['product_list.rs', 'warehouse_list.rs', 'department_list.rs']:
        path = os.path.join('/home/root0/桌面/121/1/frontend/src/pages', page)
        with open(path, 'r') as f:
            content = f.read()
        
        # product_list.rs uses `products.set(data)` where `data` is `ProductListResponse`
        content = content.replace("products.set(data);", "products.set(data.data);")
        content = content.replace("warehouses.set(data);", "warehouses.set(data.data);")
        content = content.replace("departments.set(data);", "departments.set(data.data);")
        
        with open(path, 'w') as f:
            f.write(content)

def fix_callbacks():
    for page in ['purchase_price.rs', 'sales_price.rs', 'quality_inspection.rs', 'product_category.rs', 'inventory_stock.rs']:
        path = os.path.join('/home/root0/桌面/121/1/frontend/src/pages', page)
        with open(path, 'r') as f:
            content = f.read()
        
        content = content.replace("onclick={load_data.clone()}", "onclick={load_data.reform(|_| ())}")
        
        with open(path, 'w') as f:
            f.write(content)

def fix_sales_order():
    path = '/home/root0/桌面/121/1/frontend/src/pages/sales_order.rs'
    with open(path, 'r') as f:
        content = f.read()
    
    content = content.replace("self.filter_status", "self.state.status_filter")
    content = content.replace("Some(Msg::UpdateShipItemWarehouse(idx, 0))", "Msg::UpdateShipItemWarehouse(idx, 0)")
    
    with open(path, 'w') as f:
        f.write(content)

def fix_customer_credit():
    path = '/home/root0/桌面/121/1/frontend/src/pages/customer_credit.rs'
    with open(path, 'r') as f:
        content = f.read()
    
    content = content.replace("CustomerCreditService::list_with_query", "CustomerCreditService::list")
    
    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    fix_responses()
    fix_callbacks()
    fix_sales_order()
    fix_customer_credit()
    print("Fixed remaining compile errors.")
