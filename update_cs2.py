import re

def update_page(file_path, entity_type, page_title):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    start_str = f'<MainLayout current_page={{"{page_title}"}}>'
    end_str = '</MainLayout>'
    
    start_idx = content.find(start_str)
    end_idx = content.find(end_str, start_idx) + len(end_str)

    if start_idx == -1 or end_idx == -1 + len(end_str):
        print(f"Could not find MainLayout tags in {file_path}")
        return

    if entity_type == 'customer':
        new_html = f"""<MainLayout current_page={{"客户管理"}}>
                <div class="space-y-4">
                    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                        <h1 class="text-[18px] font-bold text-[#1D2129]">{{"客户管理"}}</h1>
                        <div class="flex items-center gap-2 overflow-x-auto pb-2 md:pb-0">
                            <button class="btn-primary shrink-0" onclick={{ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}}>{{"新增客户"}}</button>
                            <button class="btn-secondary shrink-0">{{"导入"}}</button>
                            <button class="btn-text shrink-0">{{"导出"}}</button>
                            <button class="btn-text shrink-0" onclick={{ctx.link().callback(|_| Msg::LoadCustomers)}}>{{"刷新"}}</button>
                        </div>
                    </div>

                    <div class="card p-4 flex flex-wrap gap-3 items-center">
                        <div class="w-full md:w-[200px]">
                            <input type="text" placeholder="客户名称/编号" value={{self.keyword.clone()}} onchange={{on_keyword_change}} />
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select onchange={{on_type_change}} value={{self.filter_type.clone()}} class="text-[#86909C]">
                                <option value="">{{"客户类型"}}</option>
                                <option value="retail">{{"零售"}}</option>
                                <option value="wholesale">{{"批发"}}</option>
                                <option value="distributor">{{"分销商"}}</option>
                            </select>
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select onchange={{on_status_change}} value={{self.filter_status.clone()}} class="text-[#86909C]">
                                <option value="">{{"合作状态"}}</option>
                                <option value="active">{{"活跃"}}</option>
                                <option value="inactive">{{"流失"}}</option>
                                <option value="blacklisted">{{"黑名单"}}</option>
                            </select>
                        </div>
                        <button class="btn-primary" onclick={{ctx.link().callback(|_| Msg::LoadCustomers)}}>{{"查询"}}</button>
                    </div>

                    <div class="card p-0 overflow-hidden">
                        <div class="table-responsive hidden md:block">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th class="w-12 text-center">{{"序号"}}</th>
                                        <th>{{"客户编号"}}</th>
                                        <th>{{"客户名称"}}</th>
                                        <th>{{"联系人"}}</th>
                                        <th>{{"联系方式"}}</th>
                                        <th>{{"客户类型"}}</th>
                                        <th>{{"合作状态"}}</th>
                                        <th class="text-right">{{"应收余额"}}</th>
                                        <th class="text-center">{{"操作"}}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {{
                                        if self.loading {{
                                            html! {{ <tr><td colspan="9" class="text-center py-10 text-[#86909C]"><div class="loading-spinner w-6 h-6 mx-auto"></div></td></tr> }}
                                        }} else if self.customers.is_empty() {{
                                            html! {{ <tr><td colspan="9" class="text-center py-10 text-[#86909C]">{{"暂无数据"}}</td></tr> }}
                                        }} else {{
                                            html! {{
                                                for self.customers.iter().enumerate().map(|(index, item)| {{
                                                    let item_clone = item.clone();
                                                    let id = item.id;
                                                    html! {{
                                                        <tr>
                                                            <td class="text-center text-[#86909C]">{{index + 1}}</td>
                                                            <td class="font-bold text-[#1D2129]">{{&item.code}}</td>
                                                            <td>{{&item.name}}</td>
                                                            <td>{{item.contact_person.as_deref().unwrap_or("-")}}</td>
                                                            <td>{{item.contact_phone.as_deref().unwrap_or("-")}}</td>
                                                            <td><span class="badge-info">{{&item.customer_type}}</span></td>
                                                            <td>
                                                                if item.status == "active" {{
                                                                    <span class="status-success">{{"活跃"}}</span>
                                                                }} else {{
                                                                    <span class="status-danger">{{"未活跃"}}</span>
                                                                }}
                                                            </td>
                                                            <td class="text-right text-[#F53F3F] font-bold">{{"¥"}}{{item.credit_amount.as_deref().unwrap_or("0.00")}}</td>
                                                            <td>
                                                                <div class="flex items-center justify-center gap-2">
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{{"查看"}}</button>
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::OpenModal(ModalMode::Edit, Some(item_clone.clone())))}}>{{"编辑"}}</button>
                                                                    <button class="text-[#F53F3F] hover:text-[#E03535] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::DeleteCustomer(id))}}>{{"删除"}}</button>
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
                                }} else if self.customers.is_empty() {{
                                    html! {{ <div class="text-center py-10 text-[#86909C]">{{"暂无数据"}}</div> }}
                                }} else {{
                                    html! {{
                                        for self.customers.iter().map(|item| {{
                                            let item_clone = item.clone();
                                            let id = item.id;
                                            html! {{
                                                <div class="card p-4">
                                                    <div class="flex justify-between items-start mb-2">
                                                        <div>
                                                            <div class="font-bold text-[#1D2129] text-[14px]">{{&item.name}}</div>
                                                            <div class="text-[12px] text-[#86909C] mt-0.5">{{&item.code}}</div>
                                                        </div>
                                                        if item.status == "active" {{
                                                            <span class="status-success">{{"活跃"}}</span>
                                                        }} else {{
                                                            <span class="status-danger">{{"未活跃"}}</span>
                                                        }}
                                                    </div>
                                                    <div class="text-[12px] text-[#4E5969] mb-2">
                                                        <div>{{"联系人: "}}{{item.contact_person.as_deref().unwrap_or("-")}}{{" "}}{{item.contact_phone.as_deref().unwrap_or("")}}</div>
                                                    </div>
                                                    <div class="flex justify-between items-end mt-3 pt-3 border-t border-[#E5E6EB]">
                                                        <div class="text-[12px] text-[#4E5969]">{{"应收: "}}<span class="text-[#F53F3F] font-bold">{{"¥"}}{{item.credit_amount.as_deref().unwrap_or("0.00")}}</span></div>
                                                        <div class="flex gap-2">
                                                            <button class="text-[#165DFF] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::OpenModal(ModalMode::Edit, Some(item_clone.clone())))}}>{{"编辑"}}</button>
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
                            <div class="text-[#86909C]">{{"共 "}}{{self.customers.len()}}{{" 条记录"}}</div>
                            <div class="flex items-center gap-2">
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={{ctx.link().callback(|_| Msg::ChangePage(1))}}>{{"上一页"}}</button>
                                <span class="text-[#165DFF] bg-[#E8F3FF] px-3 py-1 rounded">{{self.page}}</span>
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={{ctx.link().callback(|_| Msg::ChangePage(2))}}>{{"下一页"}}</button>
                            </div>
                        </div>
                    </div>

                    if self.show_modal {{
                        {{ self.render_modal(ctx) }}
                    }}
                </div>
            </MainLayout>"""
    else:
        new_html = f"""<MainLayout current_page={{"供应商管理"}}>
                <div class="space-y-4">
                    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                        <h1 class="text-[18px] font-bold text-[#1D2129]">{{"供应商管理"}}</h1>
                        <div class="flex items-center gap-2 overflow-x-auto pb-2 md:pb-0">
                            <button class="btn-primary shrink-0" onclick={{ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}}>{{"新增供应商"}}</button>
                            <button class="btn-secondary shrink-0">{{"导入"}}</button>
                            <button class="btn-text shrink-0">{{"导出"}}</button>
                            <button class="btn-text shrink-0" onclick={{ctx.link().callback(|_| Msg::LoadSuppliers)}}>{{"刷新"}}</button>
                        </div>
                    </div>

                    <div class="card p-4 flex flex-wrap gap-3 items-center">
                        <div class="w-full md:w-[200px]">
                            <input type="text" placeholder="供应商名称/编号" value={{self.keyword.clone()}} onchange={{on_keyword_change}} />
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select value={{self.filter_category.clone()}} class="text-[#86909C]">
                                <option value="">{{"供应品类"}}</option>
                                <option value="knit">{{"针织"}}</option>
                                <option value="woven">{{"梭织"}}</option>
                            </select>
                        </div>
                        <div class="w-full md:w-[150px]">
                            <select onchange={{on_status_change}} value={{self.filter_status.clone()}} class="text-[#86909C]">
                                <option value="">{{"合作状态"}}</option>
                                <option value="active">{{"活跃"}}</option>
                                <option value="inactive">{{"流失"}}</option>
                                <option value="blacklisted">{{"黑名单"}}</option>
                            </select>
                        </div>
                        <button class="btn-primary" onclick={{ctx.link().callback(|_| Msg::LoadSuppliers)}}>{{"查询"}}</button>
                    </div>

                    <div class="card p-0 overflow-hidden">
                        <div class="table-responsive hidden md:block">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th class="w-12 text-center">{{"序号"}}</th>
                                        <th>{{"供应商编号"}}</th>
                                        <th>{{"供应商名称"}}</th>
                                        <th>{{"联系人"}}</th>
                                        <th>{{"联系方式"}}</th>
                                        <th>{{"供应品类"}}</th>
                                        <th>{{"合作状态"}}</th>
                                        <th class="text-right">{{"应付余额"}}</th>
                                        <th class="text-center">{{"操作"}}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {{
                                        if self.loading {{
                                            html! {{ <tr><td colspan="9" class="text-center py-10 text-[#86909C]"><div class="loading-spinner w-6 h-6 mx-auto"></div></td></tr> }}
                                        }} else if self.suppliers.is_empty() {{
                                            html! {{ <tr><td colspan="9" class="text-center py-10 text-[#86909C]">{{"暂无数据"}}</td></tr> }}
                                        }} else {{
                                            html! {{
                                                for self.suppliers.iter().enumerate().map(|(index, item)| {{
                                                    let item_clone = item.clone();
                                                    let id = item.id;
                                                    let is_knit = index % 2 == 0;
                                                    let badge_class = if is_knit {{ "badge-knit" }} else {{ "badge-woven" }};
                                                    let badge_text = if is_knit {{ "针织" }} else {{ "梭织" }};
                                                    html! {{
                                                        <tr>
                                                            <td class="text-center text-[#86909C]">{{index + 1}}</td>
                                                            <td class="font-bold text-[#1D2129]">{{&item.code}}</td>
                                                            <td>{{&item.name}}</td>
                                                            <td>{{item.contact_person.as_deref().unwrap_or("-")}}</td>
                                                            <td>{{item.contact_phone.as_deref().unwrap_or("-")}}</td>
                                                            <td><span class={{format!("px-1.5 py-0.5 rounded text-[10px] {{}}", badge_class)}}>{{badge_text}}</span></td>
                                                            <td>
                                                                if item.status == "active" {{
                                                                    <span class="status-success">{{"活跃"}}</span>
                                                                }} else {{
                                                                    <span class="status-danger">{{"未活跃"}}</span>
                                                                }}
                                                            </td>
                                                            <td class="text-right text-[#F53F3F] font-bold">{{"¥"}}{{item.credit_amount.as_deref().unwrap_or("0.00")}}</td>
                                                            <td>
                                                                <div class="flex items-center justify-center gap-2">
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{{"查看"}}</button>
                                                                    <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::OpenModal(ModalMode::Edit, Some(item_clone.clone())))}}>{{"编辑"}}</button>
                                                                    <button class="text-[#F53F3F] hover:text-[#E03535] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::DeleteSupplier(id))}}>{{"删除"}}</button>
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
                                }} else if self.suppliers.is_empty() {{
                                    html! {{ <div class="text-center py-10 text-[#86909C]">{{"暂无数据"}}</div> }}
                                }} else {{
                                    html! {{
                                        for self.suppliers.iter().enumerate().map(|(index, item)| {{
                                            let item_clone = item.clone();
                                            let id = item.id;
                                            let is_knit = index % 2 == 0;
                                            let badge_class = if is_knit {{ "badge-knit" }} else {{ "badge-woven" }};
                                            let badge_text = if is_knit {{ "针织" }} else {{ "梭织" }};
                                            html! {{
                                                <div class="card p-4">
                                                    <div class="flex justify-between items-start mb-2">
                                                        <div>
                                                            <div class="font-bold text-[#1D2129] text-[14px]">{{&item.name}}</div>
                                                            <div class="text-[12px] text-[#86909C] mt-0.5">{{&item.code}}</div>
                                                        </div>
                                                        <div class="flex flex-col items-end gap-1">
                                                            if item.status == "active" {{
                                                                <span class="status-success">{{"活跃"}}</span>
                                                            }} else {{
                                                                <span class="status-danger">{{"未活跃"}}</span>
                                                            }}
                                                            <span class={{format!("px-1.5 py-0.5 rounded text-[10px] {{}}", badge_class)}}>{{badge_text}}</span>
                                                        </div>
                                                    </div>
                                                    <div class="text-[12px] text-[#4E5969] mb-2">
                                                        <div>{{"联系人: "}}{{item.contact_person.as_deref().unwrap_or("-")}}{{" "}}{{item.contact_phone.as_deref().unwrap_or("")}}</div>
                                                    </div>
                                                    <div class="flex justify-between items-end mt-3 pt-3 border-t border-[#E5E6EB]">
                                                        <div class="text-[12px] text-[#4E5969]">{{"应付: "}}<span class="text-[#F53F3F] font-bold">{{"¥"}}{{item.credit_amount.as_deref().unwrap_or("0.00")}}</span></div>
                                                        <div class="flex gap-2">
                                                            <button class="text-[#165DFF] text-[14px]" onclick={{ctx.link().callback(move |_| Msg::OpenModal(ModalMode::Edit, Some(item_clone.clone())))}}>{{"编辑"}}</button>
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
                            <div class="text-[#86909C]">{{"共 "}}{{self.suppliers.len()}}{{" 条记录"}}</div>
                            <div class="flex items-center gap-2">
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={{ctx.link().callback(|_| Msg::ChangePage(1))}}>{{"上一页"}}</button>
                                <span class="text-[#165DFF] bg-[#E8F3FF] px-3 py-1 rounded">{{self.page}}</span>
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]" onclick={{ctx.link().callback(|_| Msg::ChangePage(2))}}>{{"下一页"}}</button>
                            </div>
                        </div>
                    </div>

                    if self.show_modal {{
                        {{ self.render_modal(ctx) }}
                    }}
                </div>
            </MainLayout>"""

    new_content = content[:start_idx] + new_html + content[end_idx:]
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(new_content)

update_page('frontend/src/pages/customer.rs', 'customer', '客户管理')
update_page('frontend/src/pages/supplier.rs', 'supplier', '供应商管理')
print("Replaced content safely via exact bounds.")
