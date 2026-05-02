import os
import re

def fix_role_list():
    path = '/home/root0/桌面/121/1/frontend/src/pages/role_list.rs'
    with open(path, 'r') as f:
        content = f.read()
    
    # replace e.target()?.dyn_into... ok()?; with proper pattern
    pattern = r"let input = e\.target\(\)\?\.dyn_into::<web_sys::(Html\w+Element)>\(\)\.ok\(\)\?;"
    # we want to do:
    # let target = e.target()?;
    # let input = target.dyn_into::<web_sys::HtmlInputElement>().ok()?;
    
    # Since it's a bit hard to regex replace, let's just do text replacement
    # Wait, the error is about ? in closure returning Msg.
    # The closure needs to return Option<Msg> and ? on Option works IF it returns Option.
    # Ah, the closure returns `Option<Msg>`, but the `?` returns `None`. The closure signature is inferred.
    # Let's change `link.batch_callback(|e: Event| {` to `link.batch_callback(|e: Event| -> Option<Msg> {`
    content = content.replace("link.batch_callback(|e: Event| {", "link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {")
    with open(path, 'w') as f:
        f.write(content)

def fix_lifetimes():
    for page in ['product_list.rs', 'warehouse_list.rs', 'department_list.rs']:
        path = os.path.join('/home/root0/桌面/121/1/frontend/src/pages', page)
        with open(path, 'r') as f:
            content = f.read()
        content = re.sub(r'on_delete\.reform\(move \|_\| ([a-z])\.id\)', r'{ let id = \1.id; on_delete.reform(move |_| id) }', content)
        with open(path, 'w') as f:
            f.write(content)

def fix_inventory_stock():
    path = '/home/root0/桌面/121/1/frontend/src/pages/inventory_stock.rs'
    with open(path, 'r') as f:
        content = f.read()
    content = content.replace("let b = &*batch_no;", "let b = (*batch_no).clone();")
    content = content.replace("let c = &*color_no;", "let c = (*color_no).clone();")
    content = content.replace("Some(b.to_string())", "Some(b)")
    content = content.replace("Some(c.to_string())", "Some(c)")
    with open(path, 'w') as f:
        f.write(content)

def fix_sales_order():
    path = '/home/root0/桌面/121/1/frontend/src/pages/sales_order.rs'
    with open(path, 'r') as f:
        content = f.read()
    # It says no field `filter_status` on type `&mut SalesOrderPage`.
    # Let's check the fields. It has `status_filter`.
    content = content.replace("self.filter_status", "self.status_filter")
    content = content.replace("Msg::UpdateShipItemWarehouse(idx, 0)", "Some(Msg::UpdateShipItemWarehouse(idx, 0))")
    with open(path, 'w') as f:
        f.write(content)

def fix_customer_credit():
    path = '/home/root0/桌面/121/1/frontend/src/pages/customer_credit.rs'
    with open(path, 'r') as f:
        content = f.read()
    content = content.replace("CustomerCreditService::list_with_query", "CustomerCreditService::list_credits")
    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    fix_role_list()
    fix_lifetimes()
    fix_inventory_stock()
    fix_sales_order()
    fix_customer_credit()
    print("Fixed round 3.")
