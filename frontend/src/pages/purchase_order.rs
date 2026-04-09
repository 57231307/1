//! 采购订单管理页面

use crate::components::main_layout::MainLayout;
use crate::models::purchase_order::{PurchaseOrder, PurchaseOrderQuery};
use crate::services::purchase_order_service::PurchaseOrderService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct PurchaseOrderPage {
    printing_order: Option<crate::models::purchase_order::PurchaseOrder>,
    print_trigger: bool,
    orders: Vec<PurchaseOrder>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadOrders,
    OrdersLoaded(Vec<PurchaseOrder>),
    LoadError(String),
    SetFilterStatus(String),
    ViewOrder(i32),
    DeleteOrder(i32),
    SubmitOrder(i32),
    PrintOrder(crate::models::purchase_order::PurchaseOrder),
    ClearPrint,
    ApproveOrder(i32),
    RejectOrder(i32),
    CloseOrder(i32),
    ChangePage(u64),
}

impl Component for PurchaseOrderPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            orders: Vec::new(),
            loading: true,
            printing_order: None,
            print_trigger: false,
            error: None,
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadOrders);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadOrders => {
                self.loading = true;
                let query = PurchaseOrderQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    supplier_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::list(query).await {
                        Ok(orders) => link.send_message(Msg::OrdersLoaded(orders)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::OrdersLoaded(orders) => {
                self.orders = orders;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                ctx.link().send_message(Msg::LoadOrders);
                false
            }
            Msg::ViewOrder(id) => {
                web_sys::window()
                    .unwrap()
                    .location()
                    .set_href(&format!("/purchase-orders/{}", id))
                    .ok();
                false
            }
            Msg::DeleteOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }

            Msg::PrintOrder(order) => {
                self.printing_order = Some(order);
                self.print_trigger = true;
                true
            }
            Msg::ClearPrint => {
                self.printing_order = None;
                self.print_trigger = false;
                true
            }
            Msg::SubmitOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::submit(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::approve(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RejectOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::reject(id, "不符合要求".to_string()).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CloseOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::close(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadOrders);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <MainLayout current_page={"采购订单"}>
                <div class="space-y-4">
                    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                        <h1 class="text-[18px] font-bold text-[#1D2129]">{"采购订单管理"}</h1>
                        <div class="flex items-center gap-2 overflow-x-auto pb-2 md:pb-0">
                            <button class="btn-primary shrink-0">{"新增订单"}</button>
                            <button class="btn-secondary shrink-0">{"导入订单"}</button>
                            <button class="btn-secondary shrink-0">{"批量审核"}</button>
                            <button class="btn-secondary shrink-0">{"批量打印"}</button>
                            <button class="btn-text shrink-0">{"导出"}</button>
                            <button class="btn-text shrink-0" onclick={ctx.link().callback(|_| Msg::LoadOrders)}>{"刷新"}</button>
                        </div>
                    </div>

                    <div class="card p-4 flex flex-wrap gap-3 items-center">
                        <div class="w-full md:w-[200px]">
                            <input type="text" placeholder="订单号" />
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select class="text-[#86909C]">
                                <option value="">{"供应商"}</option>
                            </select>
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select value={self.filter_status.clone()} onchange={on_status_change} class="text-[#86909C]">
                                <option value="">{"订单状态"}</option>
                                <option value="草稿">{"草稿"}</option>
                                <option value="待审批">{"待审核"}</option>
                                <option value="已审批">{"已审核"}</option>
                                <option value="部分到货">{"已部分到货"}</option>
                                <option value="全部到货">{"已全部到货"}</option>
                                <option value="已关闭">{"已关闭"}</option>
                                <option value="作废">{"作废"}</option>
                            </select>
                        </div>
                        <div class="w-full md:w-[200px] flex items-center gap-2">
                            <input type="date" class="w-full text-[#86909C]" />
                            <span class="text-[#86909C]">{"-"}</span>
                            <input type="date" class="w-full text-[#86909C]" />
                        </div>
                        <div class="w-full md:w-[150px]">
                            <input type="text" placeholder="面料关键词" />
                        </div>
                        <div class="w-full md:w-[120px]">
                            <select class="text-[#86909C]">
                                <option value="">{"品类"}</option>
                                <option value="knit">{"针织"}</option>
                                <option value="woven">{"梭织"}</option>
                            </select>
                        </div>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadOrders)}>{"查询"}</button>
                    </div>

                    <div class="card p-0 overflow-hidden">
                        <div class="table-responsive hidden md:block">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th class="w-12 text-center">{"序号"}</th>
                                        <th>{"订单号"}</th>
                                        <th>{"供应商"}</th>
                                        <th>{"采购日期"}</th>
                                        <th>{"品类"}</th>
                                        <th class="text-right">{"总数量"}</th>
                                        <th class="text-right">{"总金额"}</th>
                                        <th>{"状态"}</th>
                                        <th class="text-center">{"操作"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {
                                        if self.loading {
                                            html! { <tr><td colspan="9" class="text-center py-10 text-[#86909C]"><div class="loading-spinner w-6 h-6 mx-auto"></div></td></tr> }
                                        } else if self.orders.is_empty() {
                                            html! { <tr><td colspan="9" class="text-center py-10 text-[#86909C]">{"暂无数据"}</td></tr> }
                                        } else {
                                            html! {
                                                for self.orders.iter().enumerate().map(|(index, order)| {
                                                    let is_knit = index % 2 == 0;
                                                    let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                                    let badge_text = if is_knit { "针织" } else { "梭织" };
                                                    let status_class = match order.status.as_str() {
                                                        "草稿" => "status-draft",
                                                        "待审批" => "status-warning",
                                                        "已审批" => "status-info",
                                                        "部分到货" => "status-warning text-opacity-80",
                                                        "全部到货" => "status-success",
                                                        "已关闭" => "status-draft",
                                                        "作废" => "status-danger",
                                                        _ => "status-draft",
                                                    };
                                                    html! {
                                                        <tr>
                                                            <td class="text-center text-[#86909C]">{index + 1}</td>
                                                            <td class="font-bold text-[#1D2129]">{&order.order_no}</td>
                                                            <td>{"供应商 A"}</td>
                                                            <td>{"2023-10-25"}</td>
                                                            <td><span class={format!("px-1.5 py-0.5 rounded text-[10px] {}", badge_class)}>{badge_text}</span></td>
                                                            <td class="text-right">{order.total_quantity.as_deref().unwrap_or("0")}{" kg"}</td>
                                                            <td class="text-right text-[#F53F3F] font-bold">{"¥"}{&order.total_amount}</td>
                                                            <td><span class={format!("badge {}", status_class)}>{&order.status}</span></td>
                                                            <td>
                                                                <div class="flex items-center justify-center gap-2">
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{"查看"}</button>
                                                                    if order.status == "待审批" {
                                                                        <button class="text-[#00B42A] hover:text-[#009A22] text-[14px]" onclick={ctx.link().callback({ let id = order.id; move |_| Msg::ApproveOrder(id) })}>{"审核"}</button>
                                                                    }
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{"打印"}</button>
                                                                </div>
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                            }
                                        }
                                    }
                                </tbody>
                            </table>
                        </div>

                        
                        <div class="md:hidden grid grid-cols-1 gap-3 p-3 bg-[#F5F7FA]">
                            {
                                if self.loading {
                                    html! { <div class="text-center py-10 text-[#86909C]"><div class="loading-spinner w-6 h-6 mx-auto"></div></div> }
                                } else if self.orders.is_empty() {
                                    html! { <div class="text-center py-10 text-[#86909C]">{"暂无数据"}</div> }
                                } else {
                                    html! {
                                        for self.orders.iter().enumerate().map(|(index, order)| {
                                            let is_knit = index % 2 == 0;
                                            let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                            let badge_text = if is_knit { "针织" } else { "梭织" };
                                            let status_class = match order.status.as_str() {
                                                "草稿" => "status-draft",
                                                "待审批" => "status-warning",
                                                "已审批" => "status-info",
                                                "部分到货" => "status-warning text-opacity-80",
                                                "全部到货" => "status-success",
                                                "已关闭" => "status-draft",
                                                "作废" => "status-danger",
                                                _ => "status-draft",
                                            };
                                            html! {
                                                <div class="card p-4">
                                                    <div class="flex justify-between items-start mb-2">
                                                        <div>
                                                            <div class="font-bold text-[#1D2129] text-[14px]">{&order.order_no}</div>
                                                            <div class="text-[12px] text-[#86909C] mt-0.5">{"供应商 A"}</div>
                                                        </div>
                                                        <span class={format!("badge {}", status_class)}>{&order.status}</span>
                                                    </div>
                                                    <div class="text-[12px] text-[#4E5969] mb-2 flex justify-between">
                                                        <div>{"采购数量: "}{order.total_quantity.as_deref().unwrap_or("0")}{" kg"}</div>
                                                        <span class={format!("px-1.5 py-0.5 rounded text-[10px] {}", badge_class)}>{badge_text}</span>
                                                    </div>
                                                    <div class="flex justify-between items-end mt-3 pt-3 border-t border-[#E5E6EB]">
                                                        <div class="text-[12px] text-[#4E5969]">{"总金额: "}<span class="text-[#F53F3F] font-bold">{"¥"}{&order.total_amount}</span></div>
                                                        <div class="flex gap-2">
                                                            <button class="text-[#165DFF] text-[14px]">{"查看"}</button>
                                                            if order.status == "待审批" {
                                                                <button class="text-[#00B42A] text-[14px]" onclick={ctx.link().callback({ let id = order.id; move |_| Msg::ApproveOrder(id) })}>{"审核"}</button>
                                                            }
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        })
                                    }
                                }
                            }
                        </div>

                        
                        <div class="p-4 border-t border-[#E5E6EB] flex justify-between items-center text-[14px]">
                            <div class="text-[#86909C]">{"共 "}{self.orders.len()}{" 条记录"}</div>
                            <div class="flex items-center gap-2">
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={ctx.link().callback(|_| Msg::ChangePage(1))}>{"上一页"}</button>
                                <span class="text-[#165DFF] bg-[#E8F3FF] px-3 py-1 rounded">{self.page}</span>
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={ctx.link().callback(|_| Msg::ChangePage(2))}>{"下一页"}</button>
                            </div>
                        </div>
                    </div>
                </div>
            </MainLayout>}
    }
}

impl PurchaseOrderPage {
    fn render_print_view(&self) -> Html {
        if let Some(order) = &self.printing_order {
            html! {
                <div class="print-view" style="display: none;">
                    <div class="print-header">
                        <h2>{"秉羲管理系统 - 采购订单"}</h2>
                    </div>
                    <div class="print-info-grid">
                        <div><strong>{"订单编号："}</strong> {&order.order_no}</div>
                        <div><strong>{"订单日期："}</strong> {&order.order_date}</div>
                        <div><strong>{"供应商："}</strong> {order.supplier_name.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"要求交货期："}</strong> {order.expected_delivery_date.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"采购总金额："}</strong> {&order.total_amount} {order.currency.as_deref().unwrap_or("")}</div>
                        <div><strong>{"采购员："}</strong> {order.purchaser_id.unwrap_or(0)}</div>
                    </div>
                    <table class="print-table">
                        <thead>
                            <tr>
                                <th class="numeric-cell text-right">{"序号"}</th>
                                <th>{"产品名称"}</th>
                                <th>{"规格"}</th>
                                <th class="numeric-cell text-right">{"数量"}</th>
                                <th class="numeric-cell text-right">{"单价"}</th>
                                <th>{"强力要求"}</th>
                                <th>{"条干均匀度"}</th>
                                <th>{"色牢度级别"}</th>
                                <th class="numeric-cell text-right">{"小计"}</th>
                                <th>{"备注"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            // 实际项目中这里应该渲染 items，但目前 purchase_order.rs 的列表中没有展开 items
                            // 所以留出空行或仅打印主表信息
                            <tr>
                                <td colspan="10" style="text-align: center; padding: 20px;">{"【订单明细请在详情页查看并打印】"}</td>
                            </tr>
                        </tbody>
                    </table>
                    <div class="print-footer">
                        <div class="print-signature">{"制单人签字"}</div>
                        <div class="print-signature">{"审批人签字"}</div>
                        <div class="print-signature">{"供应商确认盖章"}</div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn render_content(&self, ctx: &Context<Self>) -> Html {
        if self.loading {
            return html! {
                <div class="loading-container">
                    <div class="spinner"></div>
                    <p>{"加载中..."}</p>
                </div>
            };
        }

        if let Some(error) = &self.error {
            return html! {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{error}</p>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadOrders)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.orders.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📦"}</div>
                    <p>{"暂无采购订单"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full">
                    <thead>
                        <tr>
                            <th>{"订单编号"}</th>
                            <th>{"供应商"}</th>
                            <th>{"订单日期"}</th>
                            <th>{"要求交货日期"}</th>
                            <th>{"强力要求"}</th>
                            <th>{"条干均匀度"}</th>
                            <th>{"色牢度级别"}</th>
                            <th>{"订单状态"}</th>
                            <th class="numeric-cell text-right">{"总金额"}</th>
                            <th>{"仓库"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.orders.iter().map(|order| {
                            let status = order.status.clone();
                            let order_id = order.id;
                            let status_check = status.clone();
                            html! {
                                <tr>
                                    <td>{&order.order_no}</td>
                                    <td>{order.supplier_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&order.order_date}</td>
                                    <td>{order.expected_delivery_date.as_deref().unwrap_or("-")}</td>
                                    <td>{"-"}</td>
                                    <td>{"-"}</td>
                                    <td>{"-"}</td>
                                    <td><span class="status-badge">{status}</span></td>
                                    <td class="numeric-cell text-right">{&order.total_amount}</td>
                                    <td>{order.warehouse_name.as_deref().unwrap_or("-")}</td>
                                    <td>
                                        {if status_check == "REJECTED" || status_check == "DRAFT" {
                                            html! {
                                                <button class="px-3 py-1 bg-indigo-600 text-white rounded text-xs" onclick={ctx.link().callback(move |_| Msg::SubmitOrder(order_id))}>{"提交审批"}</button>
                                            }
                                        } else if status_check == "PENDING" {
                                            html! {
                                                <>
                                                    <button class="px-3 py-1 bg-green-600 text-white rounded text-xs ml-2" onclick={ctx.link().callback({ let id = order.id; move |_| Msg::ApproveOrder(id) })}>{"审批通过"}</button>
                                                    <button class="px-3 py-1 bg-red-600 text-white rounded text-xs ml-2" onclick={ctx.link().callback(move |_| Msg::RejectOrder(order_id))}>{"驳回"}</button>
                                                </>
                                            }
                                        } else if status_check == "APPROVED" {
                                            html! {
                                                <button class="px-3 py-1 bg-yellow-600 text-white rounded text-xs ml-2" onclick={ctx.link().callback(move |_| Msg::CloseOrder(order_id))}>{"关闭订单"}</button>
                                            }
                                        } else {
                                            html! {}
                                        }}
                                        <button class="px-3 py-1 bg-gray-500 text-white rounded text-xs ml-2" onclick={let order_print = order.clone(); ctx.link().callback(move |_| Msg::PrintOrder(order_print.clone()))}>{"打印"}</button>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
</div>
            </div>
        }
    }
}
