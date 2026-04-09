import re

def update_po_page(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    start_str = f'<MainLayout current_page={{"/purchase-orders"}}>'
    end_str = '</MainLayout>'
    
    start_idx = content.find(start_str)
    end_idx = content.find(end_str, start_idx) + len(end_str)

    if start_idx == -1 or end_idx == -1 + len(end_str):
        print(f"Could not find MainLayout tags in {file_path}")
        return

    new_html = f"""<MainLayout current_page={{"采购订单"}}>
                <div class="space-y-4">
                    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                        <h1 class="text-[18px] font-bold text-[#1D2129]">{{"采购订单管理"}}</h1>
                        <div class="flex items-center gap-2 overflow-x-auto pb-2 md:pb-0">
                            <button class="btn-primary shrink-0">{{"新增订单"}}</button>
                            <button class="btn-secondary shrink-0">{{"导入订单"}}</button>
                            <button class="btn-secondary shrink-0">{{"批量审核"}}</button>
                            <button class="btn-secondary shrink-0">{{"批量打印"}}</button>
                            <button class="btn-text shrink-0">{{"导出"}}</button>
                            <button class="btn-text shrink-0" onclick={{ctx.link().callback(|_| Msg::LoadOrders)}}>{{"刷新"}}</button>
                        </div>
                    </div>

                    <div class="card p-4 flex flex-wrap gap-3 items-center">
                        <div class="w-full md:w-[200px]">
                            <input type="text" placeholder="订单号" />
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select class="text-[#86909C]">
                                <option value="">{{"供应商"}}</option>
                            </select>
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select value={{self.filter_status.clone()}} onchange={{on_status_change}} class="text-[#86909C]">
                                <option value="">{{"订单状态"}}</option>
                                <option value="草稿">{{"草稿"}}</option>
                                <option value="待审批">{{"待审核"}}</option>
                                <option value="已审批">{{"已审核"}}</option>
                                <option value="部分到货">{{"已部分到货"}}</option>
                                <option value="全部到货">{{"已全部到货"}}</option>
                                <option value="已关闭">{{"已关闭"}}</option>
                                <option value="作废">{{"作废"}}</option>
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
                                <option value="">{{"品类"}}</option>
                                <option value="knit">{{"针织"}}</option>
                                <option value="woven">{{"梭织"}}</option>
                            </select>
                        </div>
                        <button class="btn-primary" onclick={{ctx.link().callback(|_| Msg::LoadOrders)}}>{{"查询"}}</button>
                    </div>

                    <div class="card p-0 overflow-hidden">
                        <div class="table-responsive hidden md:block">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th class="w-12 text-center">{{"序号"}}</th>
                                        <th>{{"订单号"}}</th>
                                        <th>{{"供应商"}}</th>
                                        <th>{{"采购日期"}}</th>
                                        <th>{{"品类"}}</th>
                                        <th class="text-right">{{"总数量"}}</th>
                                        <th class="text-right">{{"总金额"}}</th>
                                        <th>{{"状态"}}</th>
                                        <th class="text-center">{{"操作"}}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {{
                                        if self.loading {{
                                            html! {{ <tr><td colspan="9" class="text-center py-10 text-[#86909C]"><div class="loading-spinner w-6 h-6 mx-auto"></div></td></tr> }}
                                        }} else if self.orders.is_empty() {{
                                            html! {{ <tr><td colspan="9" class="text-center py-10 text-[#86909C]">{{"暂无数据"}}</td></tr> }}
                                        }} else {{
                                            html! {{
                                                for self.orders.iter().enumerate().map(|(index, order)| {{
                                                    let is_knit = index % 2 == 0;
                                                    let badge_class = if is_knit {{ "badge-knit" }} else {{ "badge-woven" }};
                                                    let badge_text = if is_knit {{ "针织" }} else {{ "梭织" }};
                                                    let status_class = match order.status.as_str() {{
                                                        "草稿" => "status-draft",
                                                        "待审批" => "status-warning",
                                                        "已审批" => "status-info",
                                                        "部分到货" => "status-warning text-opacity-80",
                                                        "全部到货" => "status-success",
                                                        "已关闭" => "status-draft",
                                                        "作废" => "status-danger",
                                                        _ => "status-draft",
                                                    }};
                                                    html! {{
                                                        <tr>
                                                            <td class="text-center text-[#86909C]">{{index + 1}}</td>
                                                            <td class="font-bold text-[#1D2129]">{{&order.order_no}}</td>
                                                            <td>{{"供应商 A"}}</td>
                                                            <td>{{"2023-10-25"}}</td>
                                                            <td><span class={{format!("px-1.5 py-0.5 rounded text-[10px] {{}}", badge_class)}}>{{badge_text}}</span></td>
                                                            <td class="text-right">{{order.total_quantity.as_deref().unwrap_or("0")}}{{" kg"}}</td>
                                                            <td class="text-right text-[#F53F3F] font-bold">{{"¥"}}{{order.total_amount.as_deref().unwrap_or("0.00")}}</td>
                                                            <td><span class={{format!("badge {{}}", status_class)}}>{{&order.status}}</span></td>
                                                            <td>
                                                                <div class="flex items-center justify-center gap-2">
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{{"查看"}}</button>
                                                                    if order.status == "待审批" {{
                                                                        <button class="text-[#00B42A] hover:text-[#009A22] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::ApproveOrder(order.id))}}>{{"审核"}}</button>
                                                                    }}
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{{"打印"}}</button>
                                                                </div>
                                                            </td>
                                                        </tr>
                                                    }}
                                                }})
                                            }}
                                        }}
                                    }}
                                </tbody>
                            </table>
                        </div>

                        <!-- Mobile Cards View -->
                        <div class="md:hidden grid grid-cols-1 gap-3 p-3 bg-[#F5F7FA]">
                            {{
                                if self.loading {{
                                    html! {{ <div class="text-center py-10 text-[#86909C]"><div class="loading-spinner w-6 h-6 mx-auto"></div></div> }}
                                }} else if self.orders.is_empty() {{
                                    html! {{ <div class="text-center py-10 text-[#86909C]">{{"暂无数据"}}</div> }}
                                }} else {{
                                    html! {{
                                        for self.orders.iter().enumerate().map(|(index, order)| {{
                                            let is_knit = index % 2 == 0;
                                            let badge_class = if is_knit {{ "badge-knit" }} else {{ "badge-woven" }};
                                            let badge_text = if is_knit {{ "针织" }} else {{ "梭织" }};
                                            let status_class = match order.status.as_str() {{
                                                "草稿" => "status-draft",
                                                "待审批" => "status-warning",
                                                "已审批" => "status-info",
                                                "部分到货" => "status-warning text-opacity-80",
                                                "全部到货" => "status-success",
                                                "已关闭" => "status-draft",
                                                "作废" => "status-danger",
                                                _ => "status-draft",
                                            }};
                                            html! {{
                                                <div class="card p-4">
                                                    <div class="flex justify-between items-start mb-2">
                                                        <div>
                                                            <div class="font-bold text-[#1D2129] text-[14px]">{{&order.order_no}}</div>
                                                            <div class="text-[12px] text-[#86909C] mt-0.5">{{"供应商 A"}}</div>
                                                        </div>
                                                        <span class={{format!("badge {{}}", status_class)}}>{{&order.status}}</span>
                                                    </div>
                                                    <div class="text-[12px] text-[#4E5969] mb-2 flex justify-between">
                                                        <div>{{"采购数量: "}}{{order.total_quantity.as_deref().unwrap_or("0")}}{{" kg"}}</div>
                                                        <span class={{format!("px-1.5 py-0.5 rounded text-[10px] {{}}", badge_class)}}>{{badge_text}}</span>
                                                    </div>
                                                    <div class="flex justify-between items-end mt-3 pt-3 border-t border-[#E5E6EB]">
                                                        <div class="text-[12px] text-[#4E5969]">{{"总金额: "}}<span class="text-[#F53F3F] font-bold">{{"¥"}}{{order.total_amount.as_deref().unwrap_or("0.00")}}</span></div>
                                                        <div class="flex gap-2">
                                                            <button class="text-[#165DFF] text-[14px]">{{"查看"}}</button>
                                                            if order.status == "待审批" {{
                                                                <button class="text-[#00B42A] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::ApproveOrder(order.id))}}>{{"审核"}}</button>
                                                            }}
                                                        </div>
                                                    </div>
                                                </div>
                                            }}
                                        }})
                                    }}
                                }}
                            }}
                        </div>

                        <!-- Pagination -->
                        <div class="p-4 border-t border-[#E5E6EB] flex justify-between items-center text-[14px]">
                            <div class="text-[#86909C]">{{"共 "}}{{self.orders.len()}}{{" 条记录"}}</div>
                            <div class="flex items-center gap-2">
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={{ctx.link().callback(|_| Msg::ChangePage(1))}}>{{"上一页"}}</button>
                                <span class="text-[#165DFF] bg-[#E8F3FF] px-3 py-1 rounded">{{self.page}}</span>
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={{ctx.link().callback(|_| Msg::ChangePage(2))}}>{{"下一页"}}</button>
                            </div>
                        </div>
                    </div>
                </div>
            </MainLayout>"""

    new_content = content[:start_idx] + new_html + content[end_idx:]
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(new_content)

update_po_page('frontend/src/pages/purchase_order.rs')
print("Updated purchase_order.rs")
