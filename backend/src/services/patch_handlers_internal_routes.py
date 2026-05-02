import re

def replace_in_file(filepath, replacements):
    with open(filepath, "r") as f:
        content = f.read()
    for old, new in replacements.items():
        content = content.replace(old, new)
    with open(filepath, "w") as f:
        f.write(content)

replace_in_file("../handlers/sales_return_handler.rs", {
    "get(list_returns)": "get(list_sales_returns)",
    "post(create_return)": "post(create_sales_return)",
    "put(update_return)": "put(update_sales_return)",
    "post(submit_return)": "post(submit_sales_return)",
    "post(approve_return)": "post(approve_sales_return)",
})

replace_in_file("../handlers/purchase_return_handler.rs", {
    "get(list_returns)": "get(list_purchase_returns)",
    "post(create_return)": "post(create_purchase_return)",
    "put(update_return)": "put(update_purchase_return)",
    "post(submit_return)": "post(submit_purchase_return)",
    "post(approve_return)": "post(approve_purchase_return)",
    "post(reject_return)": "post(reject_purchase_return)",
    "get(get_return)": "get(get_purchase_return)",
    "get(list_items)": "get(list_purchase_return_items)",
    "post(create_item)": "post(create_purchase_return_item)",
    "put(update_item)": "put(update_purchase_return_item)",
    "delete(delete_item)": "delete(delete_purchase_return_item)",
})

replace_in_file("../handlers/budget_management_handler.rs", {
    "get(list_items)": "get(list_budget_items)",
    "post(create_item)": "post(create_budget_item)",
    "get(get_item)": "get(get_budget_item)",
    "put(update_item)": "put(update_budget_item)",
    "delete(delete_item)": "delete(delete_budget_item)",
})

