with open("src/pages/fabric_order.rs", "r") as f:
    content = f.read()

form_html = """                        } else {
                            let is_edit = self.modal_mode == ModalMode::Edit;
                            let order = self.current_order.clone().unwrap_or_else(|| FabricOrder {
                                id: 0,
                                order_no: String::new(),
                                customer_id: 0,
                                customer_name: None,
                                order_date: String::new(),
                                required_date: String::new(),
                                status: "待审批".to_string(),
                                total_amount: "0".to_string(),
                                paid_amount: "0".to_string(),
                                shipping_address: None,
                                delivery_address: None,
                                payment_terms: None,
                                remarks: None,
                                batch_no: None,
                                color_no: None,
                                dye_lot_no: None,
                                grade: None,
                                packaging_requirement: None,
                                quality_standard: None,
                                created_at: String::new(),
                                updated_at: String::new(),
                            });
                            
                            html! {
                                <div>
                                    <div class="form-group">
                                        <label>{"客户 ID"}</label>
                                        <input type="number" id="customer-id" value={order.customer_id.to_string()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"订单日期"}</label>
                                        <input type="date" id="order-date" value={order.order_date.clone()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"要求交货日期"}</label>
                                        <input type="date" id="required-date" value={order.required_date.clone()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"批次号"}</label>
                                        <input type="text" id="batch-no" value={order.batch_no.clone().unwrap_or_default()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"色号"}</label>
                                        <input type="text" id="color-no" value={order.color_no.clone().unwrap_or_default()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"备注"}</label>
                                        <textarea id="remarks" value={order.remarks.clone().unwrap_or_default()}></textarea>
                                    </div>
                                </div>
                            }
                        }}"""

content = content.replace("} else {\n                            html! { <p>{\"编辑/新建功能开发中...\"}</p> }\n                        }}", form_html)

# Now update the Msg::CreateOrder logic to read from inputs
create_order_logic = """Msg::CreateOrder => {
                let get_val = |id: &str| -> String {
                    web_sys::window().unwrap().document().unwrap().get_element_by_id(id).unwrap()
                        .dyn_into::<web_sys::HtmlInputElement>().map(|e| e.value())
                        .or_else(|_| web_sys::window().unwrap().document().unwrap().get_element_by_id(id).unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().map(|e| e.value()))
                        .unwrap_or_default()
                };
                let get_opt = |id: &str| -> Option<String> {
                    let v = get_val(id);
                    if v.is_empty() { None } else { Some(v) }
                };
                
                if self.modal_mode == ModalMode::Create {
                    let req = CreateFabricOrderRequest {
                        customer_id: get_val("customer-id").parse().unwrap_or(0),
                        order_date: get_val("order-date"),
                        required_date: get_val("required-date"),
                        items: vec![],
                        shipping_address: None,
                        delivery_address: None,
                        payment_terms: None,
                        remarks: get_opt("remarks"),
                        batch_no: get_opt("batch-no"),
                        color_no: get_opt("color-no"),
                        dye_lot_no: None,
                        grade: None,
                        packaging_requirement: None,
                        quality_standard: None,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FabricOrderService::create(req).await {
                            Ok(_) => link.send_message(Msg::LoadOrders),
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                } else if self.modal_mode == ModalMode::Edit {
                    let id = self.current_order.as_ref().map(|o| o.id).unwrap_or(0);
                    let req = UpdateFabricOrderRequest {
                        customer_id: Some(get_val("customer-id").parse().unwrap_or(0)),
                        order_date: Some(get_val("order-date")),
                        required_date: Some(get_val("required-date")),
                        status: None,
                        shipping_address: None,
                        delivery_address: None,
                        payment_terms: None,
                        remarks: get_opt("remarks"),
                        batch_no: get_opt("batch-no"),
                        color_no: get_opt("color-no"),
                        dye_lot_no: None,
                        grade: None,
                        packaging_requirement: None,
                        quality_standard: None,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FabricOrderService::update(id, req).await {
                            Ok(_) => link.send_message(Msg::LoadOrders),
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                self.show_modal = false;
                false
            }"""

# Find Msg::CreateOrder => { ... } and replace it.
import re
match = re.search(r'(\s*Msg::CreateOrder => \{.*?\n\s*false\n\s*\})', content, re.DOTALL)
if match:
    content = content[:match.start()] + "\n            " + create_order_logic + content[match.end():]

# Add edit button to the view table
edit_btn = """                                            <button class="btn-sm btn-primary" onclick={ctx.link().callback(move |_| Msg::OpenModal(ModalMode::Edit, Some(order_clone.clone())))}>
                                                {"编辑"}
                                            </button>
                                            {if order_status == "待审批" {"""
content = content.replace('{if order_status == "待审批" {', edit_btn)

with open("src/pages/fabric_order.rs", "w") as f:
    f.write(content)
