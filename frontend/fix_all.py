import re
import os

# 1. Fix purchase_inspection.rs
with open("src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

# Let's just restore purchase_inspection.rs first and then apply the patch properly.
os.system("git checkout src/pages/purchase_inspection.rs")

with open("src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

# Add Modal rendering to the end of `view` function
view_end = content.find("                {self.render_content(ctx)}\n            </div>")
if view_end != -1:
    modal_render = """                {self.render_content(ctx)}
                
                if self.show_modal {
                    {self.render_modal(ctx)}
                }
            </div>"""
    content = content.replace("                {self.render_content(ctx)}\n            </div>", modal_render)

# Add `render_modal` method to PurchaseInspectionPage
impl_end = content.rfind("}")
if impl_end != -1:
    render_modal = """
    /// 渲染模态框
    fn render_modal(&self, ctx: &Context<PurchaseInspectionPage>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::CloseModal);
        
        match self.modal_mode {
            ModalMode::Create => html! {
                <div class="modal-overlay">
                    <div class="modal-content">
                        <div class="modal-header">
                            <h2>{"新建采购检验单"}</h2>
                            <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            <div class="form-group">
                                <label>{"入库单 ID"}</label>
                                <input type="number" id="create-receipt-id" />
                            </div>
                            <div class="form-group">
                                <label>{"供应商 ID"}</label>
                                <input type="number" id="create-supplier-id" />
                            </div>
                            <div class="form-group">
                                <label>{"检验日期"}</label>
                                <input type="date" id="create-date" />
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button class="btn-secondary" onclick={on_close.clone()}>{"取消"}</button>
                            <button class="btn-primary" onclick={ctx.link().callback(|_| {
                                let receipt_id = web_sys::window().unwrap().document().unwrap().get_element_by_id("create-receipt-id").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap_or(0);
                                let supplier_id = web_sys::window().unwrap().document().unwrap().get_element_by_id("create-supplier-id").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap_or(0);
                                let date = web_sys::window().unwrap().document().unwrap().get_element_by_id("create-date").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                                
                                Msg::CreateInspection(CreatePurchaseInspectionRequest {
                                    receipt_id,
                                    order_id: None,
                                    supplier_id,
                                    inspection_date: if date.is_empty() { "2023-01-01".to_string() } else { date },
                                    inspector_id: None,
                                    inspection_type: None,
                                    notes: None,
                                })
                            })}>{"保存"}</button>
                        </div>
                    </div>
                </div>
            },
            ModalMode::Complete => {
                let id = self.selected_inspection.as_ref().map(|i| i.id).unwrap_or(0);
                html! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <div class="modal-header">
                                <h2>{"完成检验"}</h2>
                                <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                            </div>
                            <div class="modal-body">
                                <div class="form-group">
                                    <label>{"合格数量"}</label>
                                    <input type="number" id="complete-pass-qty" />
                                </div>
                                <div class="form-group">
                                    <label>{"不合格数量"}</label>
                                    <input type="number" id="complete-reject-qty" />
                                </div>
                                <div class="form-group">
                                    <label>{"检验结果"}</label>
                                    <select id="complete-result">
                                        <option value="PASSED">{"合格"}</option>
                                        <option value="FAILED">{"不合格"}</option>
                                    </select>
                                </div>
                            </div>
                            <div class="modal-footer">
                                <button class="btn-secondary" onclick={on_close.clone()}>{"取消"}</button>
                                <button class="btn-primary" onclick={ctx.link().callback(move |_| {
                                    let pass_qty = web_sys::window().unwrap().document().unwrap().get_element_by_id("complete-pass-qty").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                                    let reject_qty = web_sys::window().unwrap().document().unwrap().get_element_by_id("complete-reject-qty").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                                    let result = web_sys::window().unwrap().document().unwrap().get_element_by_id("complete-result").unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap().value();
                                    
                                    Msg::CompleteInspection(CompleteInspectionRequest {
                                        pass_quantity: if pass_qty.is_empty() { "0".to_string() } else { pass_qty },
                                        reject_quantity: if reject_qty.is_empty() { "0".to_string() } else { reject_qty },
                                        inspection_result: result,
                                    })
                                })}>{"确认完成"}</button>
                            </div>
                        </div>
                    </div>
                }
            },
            ModalMode::View => {
                let inspection = self.selected_inspection.as_ref().unwrap();
                html! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <div class="modal-header">
                                <h2>{"检验单详情"}</h2>
                                <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                            </div>
                            <div class="modal-body">
                                <p><strong>{"检验单号: "}</strong>{&inspection.inspection_no}</p>
                                <p><strong>{"状态: "}</strong>{&inspection.result}</p>
                                <p><strong>{"合格数量: "}</strong>{&inspection.qualified_quantity}</p>
                                <p><strong>{"不合格数量: "}</strong>{&inspection.unqualified_quantity}</p>
                                <p><strong>{"备注: "}</strong>{inspection.remarks.as_deref().unwrap_or("-")}</p>
                            </div>
                            <div class="modal-footer">
                                <button class="btn-primary" onclick={on_close}>{"关闭"}</button>
                            </div>
                        </div>
                    </div>
                }
            }
        }
    }
}
"""
    content = content[:impl_end] + render_modal

with open("src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)


# 2. Fix sales_contract.rs & purchase_contract.rs (CancelContractModal issue)
# Since I previously messed up the components, I will add CancelContractModal to both.
def fix_cancel_modal(filepath):
    with open(filepath, "r") as f:
        content = f.read()
    if "pub struct CancelContractModal" not in content:
        # It has CancelContractModalProps but no CancelContractModal struct maybe?
        content = content.replace("impl Component for CancelContractModalProps {", "impl Component for CancelContractModal {")
        if "pub struct CancelContractModal {" not in content:
            content = content.replace("pub struct CancelContractModalProps {", "pub struct CancelContractModalProps {\n    pub on_close: Callback<()>,\n    pub on_confirm: Callback<String>,\n}\n\npub struct CancelContractModal {\n    pub reason: String,\n}\n\n// ")
        
        # Replace <CancelContractModalProps with <CancelContractModal
        content = content.replace("<CancelContractModalProps", "<CancelContractModal")
        with open(filepath, "w") as f:
            f.write(content)

fix_cancel_modal("src/pages/sales_contract.rs")
fix_cancel_modal("src/pages/purchase_contract.rs")

# 3. Fix purchase_contract.rs ExecuteContractRequest missing
with open("src/pages/purchase_contract.rs", "r") as f:
    content = f.read()
    content = content.replace("crate::models::purchase_contract::ExecuteContractRequest", "crate::models::purchase_contract::UpdatePurchaseContractRequest")
with open("src/pages/purchase_contract.rs", "w") as f:
    f.write(content)

# 4. Fix role_list.rs
with open("src/pages/role_list.rs", "r") as f:
    content = f.read()
    content = re.sub(r'Msg::NameChanged\((.*?)\)', r'Some(Msg::NameChanged(\1))', content)
    content = re.sub(r'Msg::CodeChanged\((.*?)\)', r'Some(Msg::CodeChanged(\1))', content)
    content = re.sub(r'Msg::DescriptionChanged\((.*?)\)', r'Some(Msg::DescriptionChanged(\1))', content)
    content = re.sub(r'Msg::IsSystemChanged\((.*?)\)', r'Some(Msg::IsSystemChanged(\1))', content)
with open("src/pages/role_list.rs", "w") as f:
    f.write(content)

# 5. Fix sales_order.rs status_filter
with open("src/pages/sales_order.rs", "r") as f:
    content = f.read()
    content = content.replace("self.status_filter", "self.filter_status")
with open("src/pages/sales_order.rs", "w") as f:
    f.write(content)

# 6. Fix customer_credit.rs
with open("src/pages/customer_credit.rs", "r") as f:
    content = f.read()
    content = content.replace("CustomerCreditService::list_credits(&params)", "CustomerCreditService::list_credits(params)")
with open("src/pages/customer_credit.rs", "w") as f:
    f.write(content)

# 7. Fix inventory_stock.rs lifetime issue
with open("src/pages/inventory_stock.rs", "r") as f:
    content = f.read()
    # We need to capture loading and filter properly
    if "let b_loading = loading.clone();" not in content:
        content = content.replace("spawn_local(async move {", "let b_loading = loading.clone();\n            spawn_local(async move {")
        content = content.replace("loading.set(false);", "b_loading.set(false);")
with open("src/pages/inventory_stock.rs", "w") as f:
    f.write(content)

