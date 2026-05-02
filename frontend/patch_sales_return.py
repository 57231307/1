import re

with open("src/pages/sales_return.rs", "r") as f:
    content = f.read()

# 1. Add new Msg variants
if "OpenCreateModal" not in content:
    content = content.replace("    CloseDetailModal,\n}", "    CloseDetailModal,\n    OpenCreateModal,\n    CloseCreateModal,\n    UpdateInput(String, String),\n    CreateReturn,\n}")

# 2. Update update() match arms
create_logic = """            Msg::OpenCreateModal => {
                self.show_modal = true;
                true
            }
            Msg::CloseCreateModal => {
                self.show_modal = false;
                true
            }
            Msg::UpdateInput(field, value) => {
                match field.as_str() {
                    "return_no" => self.new_return_no = value,
                    "customer_id" => self.new_customer_id = value,
                    "product_id" => self.new_product_id = value,
                    "quantity" => self.new_quantity = value,
                    "reason" => self.new_reason = value,
                    _ => {}
                }
                true
            }
            Msg::CreateReturn => {
                let req = CreateSalesReturnRequest {
                    return_no: self.new_return_no.clone(),
                    sales_order_id: None,
                    customer_id: self.new_customer_id.parse().unwrap_or(0),
                    return_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    warehouse_id: 1, // Default warehouse
                    reason: self.new_reason.clone(),
                    remarks: None,
                    items: vec![
                        CreateSalesReturnItemRequest {
                            product_id: self.new_product_id.parse().unwrap_or(0),
                            quantity: self.new_quantity.parse().unwrap_or_default(),
                            unit_price: None,
                        }
                    ],
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesReturnService::create(req).await {
                        Ok(_) => {
                            link.send_message(Msg::LoadReturns);
                            link.send_message(Msg::CloseCreateModal);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }"""

match = re.search(r'(\s*Msg::CloseDetailModal => \{.*?\n\s*true\n\s*\})', content, re.DOTALL)
if match:
    content = content[:match.end()] + "\n" + create_logic + content[match.end():]

# 3. Fix the button
content = content.replace(
    """<button class="btn btn-primary" onclick={Callback::from(|_| {
                            gloo_dialogs::alert("新建退货单功能开发中...");
                        })}>""",
    """<button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>"""
)

# 4. Render create modal
render_create_modal = """
    fn render_create_modal(&self, ctx: &Context<Self>) -> Html {
        if !self.show_modal {
            return html! {};
        }
        let link = ctx.link();
        
        let on_input = |field: &'static str| {
            link.batch_callback(move |e: Event| {
                use wasm_bindgen::JsCast;
                let target = e.target()?.unchecked_into::<web_sys::HtmlInputElement>();
                Some(Msg::UpdateInput(field.to_string(), target.value()))
            })
        };

        html! {
            <div class="modal-overlay">
                <div class="modal-content">
                    <div class="modal-header">
                        <h2>{"新建退货单"}</h2>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseCreateModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"退货单号"}</label>
                            <input type="text" value={self.new_return_no.clone()} onchange={on_input("return_no")} />
                        </div>
                        <div class="form-group">
                            <label>{"客户 ID"}</label>
                            <input type="number" value={self.new_customer_id.clone()} onchange={on_input("customer_id")} />
                        </div>
                        <div class="form-group">
                            <label>{"产品 ID"}</label>
                            <input type="number" value={self.new_product_id.clone()} onchange={on_input("product_id")} />
                        </div>
                        <div class="form-group">
                            <label>{"退货数量"}</label>
                            <input type="number" value={self.new_quantity.clone()} onchange={on_input("quantity")} />
                        </div>
                        <div class="form-group">
                            <label>{"退货原因"}</label>
                            <input type="text" value={self.new_reason.clone()} onchange={on_input("reason")} />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={link.callback(|_| Msg::CloseCreateModal)}>{"取消"}</button>
                        <button class="btn-primary" onclick={link.callback(|_| Msg::CreateReturn)}>{"保存"}</button>
                    </div>
                </div>
            </div>
        }
    }
"""
content = content.replace("    fn render_detail_modal(&self, ctx: &Context<Self>) -> Html {", render_create_modal + "\n    fn render_detail_modal(&self, ctx: &Context<Self>) -> Html {")

content = content.replace("{ self.render_detail_modal(ctx) }", "{ self.render_detail_modal(ctx) }\n                { self.render_create_modal(ctx) }")

with open("src/pages/sales_return.rs", "w") as f:
    f.write(content)
