import re

# 1. Update sales_return_handler.rs
with open("../handlers/sales_return_handler.rs", "r") as f:
    content = f.read()

replacements = {
    "pub async fn list_returns": "pub async fn list_sales_returns",
    "pub async fn create_return": "pub async fn create_sales_return",
    "pub async fn update_return": "pub async fn update_sales_return",
    "pub async fn submit_return": "pub async fn submit_sales_return",
    "pub async fn approve_return": "pub async fn approve_sales_return"
}
for old, new in replacements.items():
    content = content.replace(old, new)
with open("../handlers/sales_return_handler.rs", "w") as f:
    f.write(content)


# 2. Update purchase_return_handler.rs
with open("../handlers/purchase_return_handler.rs", "r") as f:
    content = f.read()

replacements = {
    "pub async fn list_returns": "pub async fn list_purchase_returns",
    "pub async fn get_return": "pub async fn get_purchase_return",
    "pub async fn create_return": "pub async fn create_purchase_return",
    "pub async fn update_return": "pub async fn update_purchase_return",
    "pub async fn submit_return": "pub async fn submit_purchase_return",
    "pub async fn approve_return": "pub async fn approve_purchase_return",
    "pub async fn reject_return": "pub async fn reject_purchase_return",
    "pub async fn list_items": "pub async fn list_purchase_return_items",
    "pub async fn create_item": "pub async fn create_purchase_return_item",
    "pub async fn update_item": "pub async fn update_purchase_return_item",
    "pub async fn delete_item": "pub async fn delete_purchase_return_item",
}
for old, new in replacements.items():
    content = content.replace(old, new)
with open("../handlers/purchase_return_handler.rs", "w") as f:
    f.write(content)


# 3. Update budget_management_handler.rs
with open("../handlers/budget_management_handler.rs", "r") as f:
    content = f.read()

replacements = {
    "pub async fn list_items": "pub async fn list_budget_items",
    "pub async fn create_item": "pub async fn create_budget_item",
    "pub async fn get_item": "pub async fn get_budget_item",
    "pub async fn update_item": "pub async fn update_budget_item",
    "pub async fn delete_item": "pub async fn delete_budget_item",
}
for old, new in replacements.items():
    content = content.replace(old, new)
with open("../handlers/budget_management_handler.rs", "w") as f:
    f.write(content)


# 4. Update routes/mod.rs
with open("../routes/mod.rs", "r") as f:
    content = f.read()

content = content.replace("sales_return_handler::list_returns", "sales_return_handler::list_sales_returns")
content = content.replace("sales_return_handler::create_return", "sales_return_handler::create_sales_return")
content = content.replace("sales_return_handler::update_return", "sales_return_handler::update_sales_return")
content = content.replace("sales_return_handler::submit_return", "sales_return_handler::submit_sales_return")
content = content.replace("sales_return_handler::approve_return", "sales_return_handler::approve_sales_return")

content = content.replace("purchase_return_handler::list_returns", "purchase_return_handler::list_purchase_returns")
content = content.replace("purchase_return_handler::get_return", "purchase_return_handler::get_purchase_return")
content = content.replace("purchase_return_handler::create_return", "purchase_return_handler::create_purchase_return")
content = content.replace("purchase_return_handler::update_return", "purchase_return_handler::update_purchase_return")
content = content.replace("purchase_return_handler::submit_return", "purchase_return_handler::submit_purchase_return")
content = content.replace("purchase_return_handler::approve_return", "purchase_return_handler::approve_purchase_return")
content = content.replace("purchase_return_handler::reject_return", "purchase_return_handler::reject_purchase_return")
content = content.replace("purchase_return_handler::list_items", "purchase_return_handler::list_purchase_return_items")
content = content.replace("purchase_return_handler::create_item", "purchase_return_handler::create_purchase_return_item")
content = content.replace("purchase_return_handler::update_item", "purchase_return_handler::update_purchase_return_item")
content = content.replace("purchase_return_handler::delete_item", "purchase_return_handler::delete_purchase_return_item")

content = content.replace("budget_management_handler::list_items", "budget_management_handler::list_budget_items")
content = content.replace("budget_management_handler::create_item", "budget_management_handler::create_budget_item")
content = content.replace("budget_management_handler::get_item", "budget_management_handler::get_budget_item")
content = content.replace("budget_management_handler::update_item", "budget_management_handler::update_budget_item")
content = content.replace("budget_management_handler::delete_item", "budget_management_handler::delete_budget_item")

with open("../routes/mod.rs", "w") as f:
    f.write(content)

