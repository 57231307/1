import sys

html = """html! {{
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
}}"""

# Let's count open and close braces
open_braces = html.count('{')
close_braces = html.count('}')
print(f"Open: {open_braces}, Close: {close_braces}")
