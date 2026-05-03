with open("../frontend/src/pages/sales_return.rs", "r") as f:
    content = f.read()

# 1. Add Print Msg
content = content.replace("    ApproveReturn(i32),\n    ChangePage(u64),", "    ApproveReturn(i32),\n    PrintReturn(crate::models::sales_return::SalesReturn),\n    ClearPrint,\n    ChangePage(u64),")

# 2. Add Print Msg handling
print_logic = """
            Msg::PrintReturn(r) => {
                self.printing_return = Some(r);
                self.print_trigger = true;
                true
            }
            Msg::ClearPrint => {
                self.printing_return = None;
                true
            }"""
content = content.replace("            Msg::ApproveReturn(id) => {", print_logic + "\n            Msg::ApproveReturn(id) => {")

# 3. Add rendered hook
rendered_hook = """
    fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
        if self.print_trigger {
            self.print_trigger = false;
            if let Some(window) = web_sys::window() {
                let _ = window.print();
                ctx.link().send_message(Msg::ClearPrint);
            }
        }
    }"""
content = content.replace("    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {", rendered_hook + "\n\n    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {")

# 4. Add Print button in view modal
content = content.replace("""<button class="btn-primary" onclick={link.callback(|_| Msg::CloseDetailModal)}>{"关闭"}</button>""", """<button class="btn-info" onclick={link.callback(move |_| Msg::PrintReturn(return_order.clone()))}>{"打印"}</button>\n                                <button class="btn-primary" onclick={link.callback(|_| Msg::CloseDetailModal)}>{"关闭"}</button>""")

# 5. Add Print styles
style = """
            <style>
                {r#"
                @media print {
                    body * {
                        visibility: hidden;
                    }
                    .modal-content, .modal-content * {
                        visibility: visible;
                    }
                    .modal-content {
                        position: absolute;
                        left: 0;
                        top: 0;
                        width: 100%;
                    }
                    .modal-footer {
                        display: none !important;
                    }
                }
                "#}
            </style>"""
content = content.replace("        html! {\n            <div class=\"sales-return-page\">", "        html! {\n            <div class=\"sales-return-page\">" + style)

with open("../frontend/src/pages/sales_return.rs", "w") as f:
    f.write(content)
