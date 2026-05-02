import os

def fix_sales_order_query():
    path = '/home/root0/桌面/121/1/frontend/src/pages/sales_order.rs'
    with open(path, 'r') as f:
        content = f.read()
    content = content.replace("crate::models::sales_order::SalesOrderQuery", "crate::models::sales_order::SalesOrderQueryParams")
    content = content.replace("self.state.status_filter", "self.filter_status")
    
    # fix batch_callback
    content = content.replace("ctx.link().batch_callback(move |e: Event| {", "ctx.link().batch_callback(move |e: Event| {")
    content = content.replace("e.target()?;", "e.target()?;")
    content = content.replace("Msg::UpdateShipItemWarehouse(idx, wid)", "Some(Msg::UpdateShipItemWarehouse(idx, wid))")
    content = content.replace("Some(Some(Msg::UpdateShipItemWarehouse(idx, 0)))", "Some(Msg::UpdateShipItemWarehouse(idx, 0))")
    
    with open(path, 'w') as f:
        f.write(content)

def fix_responses():
    for page, field in [('product_list.rs', 'products'), ('warehouse_list.rs', 'warehouses'), ('department_list.rs', 'departments')]:
        path = os.path.join('/home/root0/桌面/121/1/frontend/src/pages', page)
        with open(path, 'r') as f:
            content = f.read()
        content = content.replace("data.data", f"data.{field}")
        with open(path, 'w') as f:
            f.write(content)

def fix_contracts():
    for page in ['sales_contract.rs', 'purchase_contract.rs']:
        path = os.path.join('/home/root0/桌面/121/1/frontend/src/pages', page)
        with open(path, 'r') as f:
            content = f.read()
        content = content.replace("impl Component for CancelContractModalProps {", "impl Component for CancelContractModal {")
        content = content.replace("ExecuteContractRequest {", "crate::models::" + ("sales_contract" if "sales" in page else "purchase_contract") + "::ExecuteContractRequest {")
        with open(path, 'w') as f:
            f.write(content)

def fix_role_list():
    path = '/home/root0/桌面/121/1/frontend/src/pages/role_list.rs'
    with open(path, 'r') as f:
        content = f.read()
    # we need to replace link.callback with link.batch_callback and return Some
    content = content.replace("link.callback(|e: Event| {", "link.batch_callback(|e: Event| {")
    content = content.replace("Msg::UpdateRoleName(input.value())", "Some(Msg::UpdateRoleName(input.value()))")
    content = content.replace("Msg::UpdateRoleCode(input.value())", "Some(Msg::UpdateRoleCode(input.value()))")
    content = content.replace("Msg::UpdateRoleDescription(input.value())", "Some(Msg::UpdateRoleDescription(input.value()))")
    content = content.replace("Msg::UpdateMenuId(input.value())", "Some(Msg::UpdateMenuId(input.value()))")
    with open(path, 'w') as f:
        f.write(content)

def fix_customer_credit():
    path = '/home/root0/桌面/121/1/frontend/src/pages/customer_credit.rs'
    with open(path, 'r') as f:
        content = f.read()
    content = content.replace("CustomerCreditService::list(&params)", "CustomerCreditService::list_with_query(&params)")
    with open(path, 'w') as f:
        f.write(content)

if __name__ == "__main__":
    fix_sales_order_query()
    fix_responses()
    fix_contracts()
    fix_role_list()
    fix_customer_credit()
    print("Fixed round 2.")
