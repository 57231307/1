with open("/home/root0/桌面/121/1/frontend/src/pages/sales_order.rs", "r") as f:
    content = f.read()

# Add import
if "use crate::utils::permissions;" not in content:
    content = content.replace("use yew::prelude::*;", "use yew::prelude::*;\nuse crate::utils::permissions;")

# Replace action buttons with permission checks
# "新增销售订单" button
content = content.replace(
    '<button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>{"新增销售订单"}</button>',
    'if permissions::has_permission("sales_order", "create") { <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>{"新增销售订单"}</button> }'
)

# Edit/Delete buttons in the row
old_row_actions = """                                        <button class="btn-small btn-info" onclick={ctx.link().callback(move |_| Msg::ViewOrder(order_id))}>{"详情"}</button>
                                        <button class="btn-small btn-primary" onclick={ctx.link().callback(move |_| Msg::SubmitOrder(order_id))}>{"提交"}</button>
                                        <button class="btn-small btn-warning" onclick={ctx.link().callback(move |_| Msg::ApproveOrder(order_id))}>{"审批"}</button>
                                        <button class="btn-small btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteOrder(order_id))}>{"删除"}</button>"""

new_row_actions = """                                        <button class="btn-small btn-info" onclick={ctx.link().callback(move |_| Msg::ViewOrder(order_id))}>{"详情"}</button>
                                        if permissions::has_permission_for_resource("sales_order", "update", order_id) {
                                            <button class="btn-small btn-primary" onclick={ctx.link().callback(move |_| Msg::SubmitOrder(order_id))}>{"提交"}</button>
                                        }
                                        if permissions::has_permission_for_resource("sales_order", "approve", order_id) {
                                            <button class="btn-small btn-warning" onclick={ctx.link().callback(move |_| Msg::ApproveOrder(order_id))}>{"审批"}</button>
                                        }
                                        if permissions::has_permission_for_resource("sales_order", "delete", order_id) {
                                            <button class="btn-small btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteOrder(order_id))}>{"删除"}</button>
                                        }"""

content = content.replace(old_row_actions, new_row_actions)

with open("/home/root0/桌面/121/1/frontend/src/pages/sales_order.rs", "w") as f:
    f.write(content)
