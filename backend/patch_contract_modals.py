import re

def patch_file(file_path, execute_req_type, execute_props_type):
    with open(file_path, "r") as f:
        content = f.read()

    execute_modal = f"""
pub struct {execute_props_type} {{
    execution_type: String,
    execution_amount: String,
    related_bill_type: String,
    related_bill_id: String,
    remark: String,
}}

pub enum ExecuteMsg {{
    UpdateExecutionType(String),
    UpdateExecutionAmount(String),
    UpdateRelatedBillType(String),
    UpdateRelatedBillId(String),
    UpdateRemark(String),
    Submit,
}}

impl Component for ExecuteContractModal {{
    type Message = ExecuteMsg;
    type Properties = ExecuteContractModalProps;

    fn create(_ctx: &Context<Self>) -> Self {{
        Self {{
            execution_type: "发货".to_string(),
            execution_amount: "0".to_string(),
            related_bill_type: String::new(),
            related_bill_id: String::new(),
            remark: String::new(),
        }}
    }}

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {{
        match msg {{
            ExecuteMsg::UpdateExecutionType(val) => self.execution_type = val,
            ExecuteMsg::UpdateExecutionAmount(val) => self.execution_amount = val,
            ExecuteMsg::UpdateRelatedBillType(val) => self.related_bill_type = val,
            ExecuteMsg::UpdateRelatedBillId(val) => self.related_bill_id = val,
            ExecuteMsg::UpdateRemark(val) => self.remark = val,
            ExecuteMsg::Submit => {{
                use std::str::FromStr;
                let req = {execute_req_type} {{
                    execution_type: self.execution_type.clone(),
                    execution_amount: self.execution_amount.clone(),
                    related_bill_type: if self.related_bill_type.is_empty() {{ None }} else {{ Some(self.related_bill_type.clone()) }},
                    related_bill_id: i32::from_str(&self.related_bill_id).ok(),
                    remark: if self.remark.is_empty() {{ None }} else {{ Some(self.remark.clone()) }},
                }};
                ctx.props().on_submit.emit((ctx.props().contract_id, req));
            }}
        }}
        true
    }}

    fn view(&self, ctx: &Context<Self>) -> Html {{
        let props = ctx.props();
        
        let on_input = |f: fn(String) -> ExecuteMsg| {{
            ctx.link().callback(move |e: InputEvent| {{
                use wasm_bindgen::JsCast;
                let input = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                f(input.value())
            }})
        }};

        let on_select = |f: fn(String) -> ExecuteMsg| {{
            ctx.link().callback(move |e: Event| {{
                use wasm_bindgen::JsCast;
                let select = e.target_unchecked_into::<web_sys::HtmlSelectElement>();
                f(select.value())
            }})
        }};

        html! {{
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h2>{{format!("执行合同 #{{}}", props.contract_id)}}</h2>
                        <button onclick={{props.on_close.reform(|_| ())}}>{{"关闭"}}</button>
                    </div>
                    <div class="modal-body" style="display: flex; flex-direction: column; gap: 10px;">
                        <div>
                            <label>{{"执行类型: "}}</label>
                            <select onchange={{on_select(ExecuteMsg::UpdateExecutionType)}} value={{self.execution_type.clone()}}>
                                <option value="发货">{{"发货/收货"}}</option>
                                <option value="付款">{{"付款/收款"}}</option>
                            </select>
                        </div>
                        <div>
                            <label>{{"执行金额: "}}</label>
                            <input type="number" step="0.01" value={{self.execution_amount.clone()}} oninput={{on_input(ExecuteMsg::UpdateExecutionAmount)}} />
                        </div>
                        <div>
                            <label>{{"关联单据类型: "}}</label>
                            <input type="text" value={{self.related_bill_type.clone()}} oninput={{on_input(ExecuteMsg::UpdateRelatedBillType)}} placeholder="可选" />
                        </div>
                        <div>
                            <label>{{"关联单据ID: "}}</label>
                            <input type="number" value={{self.related_bill_id.clone()}} oninput={{on_input(ExecuteMsg::UpdateRelatedBillId)}} placeholder="可选" />
                        </div>
                        <div>
                            <label>{{"备注: "}}</label>
                            <input type="text" value={{self.remark.clone()}} oninput={{on_input(ExecuteMsg::UpdateRemark)}} placeholder="可选" />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button onclick={{props.on_close.reform(|_| ())}}>{{"取消"}}</button>
                        <button class="primary" onclick={{ctx.link().callback(|_| ExecuteMsg::Submit)}}>{{"确认执行"}}</button>
                    </div>
                </div>
            </div>
        }}
    }}
}}
"""

    cancel_modal = f"""
pub struct CancelContractModalState {{
    reason: String,
}}

pub enum CancelMsg {{
    UpdateReason(String),
    Submit,
}}

impl Component for CancelContractModal {{
    type Message = CancelMsg;
    type Properties = CancelContractModalProps;

    fn create(_ctx: &Context<Self>) -> Self {{
        Self {{ reason: String::new() }}
    }}

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {{
        match msg {{
            CancelMsg::UpdateReason(val) => self.reason = val,
            CancelMsg::Submit => {{
                ctx.props().on_submit.emit((ctx.props().contract_id, self.reason.clone()));
            }}
        }}
        true
    }}

    fn view(&self, ctx: &Context<Self>) -> Html {{
        let props = ctx.props();
        html! {{
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h2>{{format!("取消合同 #{{}}", props.contract_id)}}</h2>
                        <button onclick={{props.on_close.reform(|_| ())}}>{{"关闭"}}</button>
                    </div>
                    <div class="modal-body" style="display: flex; flex-direction: column; gap: 10px;">
                        <div>
                            <label>{{"取消原因: "}}</label>
                            <textarea 
                                value={{self.reason.clone()}} 
                                oninput={{ctx.link().callback(|e: InputEvent| {{
                                    use wasm_bindgen::JsCast;
                                    CancelMsg::UpdateReason(e.target_unchecked_into::<web_sys::HtmlTextAreaElement>().value())
                                }})}} 
                                rows="3" 
                                style="width: 100%;"
                            ></textarea>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button onclick={{props.on_close.reform(|_| ())}}>{{"关闭"}}</button>
                        <button class="danger" onclick={{ctx.link().callback(|_| CancelMsg::Submit)}}>{{"确认取消"}}</button>
                    </div>
                </div>
            </div>
        }}
    }}
}}
"""

    content = re.sub(
        r'impl Component for ExecuteContractModal \{.*?(?=\n/// 取消合同弹窗组件)',
        execute_modal,
        content,
        flags=re.DOTALL
    )

    content = re.sub(
        r'impl Component for CancelContractModal \{.*',
        cancel_modal,
        content,
        flags=re.DOTALL
    )

    # replace #[derive(Clone, PartialEq)] on modals
    content = content.replace("#[derive(Clone, PartialEq)]\npub struct ExecuteContractModal;", "")
    content = content.replace("#[derive(Clone, PartialEq)]\npub struct CancelContractModal;", "")

    with open(file_path, "w") as f:
        f.write(content)

patch_file("../frontend/src/pages/sales_contract.rs", "ExecuteSalesContractRequest", "ExecuteContractModal")
patch_file("../frontend/src/pages/purchase_contract.rs", "ExecuteContractRequest", "ExecuteContractModal")
