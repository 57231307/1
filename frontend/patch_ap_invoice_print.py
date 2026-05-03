with open("src/pages/ap_invoice.rs", "r") as f:
    content = f.read()

# 1. Add fields to struct
content = content.replace("    viewing_item: Option<ApInvoice>,}", "    viewing_item: Option<ApInvoice>,\n    printing_invoice: Option<ApInvoice>,\n    print_trigger: bool,}")

# 2. Add init
content = content.replace("            viewing_item: None,", "            viewing_item: None,\n            printing_invoice: None,\n            print_trigger: false,")

# 3. Add Print Msg
content = content.replace("    CloseModal,", "    PrintInvoice(ApInvoice),\n    ClearPrint,\n    CloseModal,")

# 4. Add Msg handling
print_logic = """
            Msg::PrintInvoice(i) => {
                self.printing_invoice = Some(i);
                self.print_trigger = true;
                true
            }
            Msg::ClearPrint => {
                self.printing_invoice = None;
                true
            }"""
content = content.replace("            Msg::Refresh => {", print_logic + "\n            Msg::Refresh => {")

# 5. Add rendered hook
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

# 6. Add Print button in view modal
content = content.replace("""<button class="btn-primary" onclick={link.callback(|_| Msg::CloseModal)}>{"关闭"}</button>""", """<button class="btn-info" onclick={link.callback(move |_| Msg::PrintInvoice(invoice.clone()))}>{"打印"}</button>\n                                <button class="btn-primary" onclick={link.callback(|_| Msg::CloseModal)}>{"关闭"}</button>""")

# 7. Add Print styles
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
content = content.replace("        html! {\n            <div class=\"ap-invoice-page\">", "        html! {\n            <div class=\"ap-invoice-page\">" + style)

with open("src/pages/ap_invoice.rs", "w") as f:
    f.write(content)
