import re

files = [
    "src/pages/ap_invoice.rs",
    "src/pages/ar_invoice.rs",
    "src/pages/sales_return.rs"
]

for filepath in files:
    with open(filepath, "r") as f:
        content = f.read()

    # Find the second rendered block
    second_rendered_start = content.find("    fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {")
    if second_rendered_start != -1:
        # we know it looks like:
        # fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
        #     if self.print_trigger {
        #         self.print_trigger = false;
        #         if let Some(window) = web_sys::window() {
        #             let _ = window.print();
        #             ctx.link().send_message(Msg::ClearPrint);
        #         }
        #     }
        # }
        end = content.find("    fn update(&mut self", second_rendered_start)
        
        # remove this block
        content = content[:second_rendered_start] + content[end:]
        
        # inject logic into first rendered block
        # fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        #     if first_render { ... }
        # }
        first_rendered_start = content.find("    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {")
        first_rendered_end = content.find("    fn update", first_rendered_start)
        
        block = content[first_rendered_start:first_rendered_end]
        block = block.replace("    }", """        if self.print_trigger {
            self.print_trigger = false;
            if let Some(window) = web_sys::window() {
                let _ = window.print();
                ctx.link().send_message(Msg::ClearPrint);
            }
        }
    }""")
        content = content[:first_rendered_start] + block + content[first_rendered_end:]

    # check Msg enum
    enum_start = content.find("pub enum Msg {")
    enum_end = content.find("}", enum_start)
    enum_block = content[enum_start:enum_end]
    if "ClearPrint" not in enum_block:
        if "ap_invoice" in filepath or "ar_invoice" in filepath:
            enum_block = enum_block.replace("    CloseModal,", "    CloseModal,\n    PrintInvoice(crate::models::ar_invoice::ArInvoice),\n    ClearPrint,")
            if "ap_invoice" in filepath:
                enum_block = enum_block.replace("crate::models::ar_invoice::ArInvoice", "crate::models::ap_invoice::ApInvoice")
        elif "sales_return" in filepath:
            enum_block = enum_block.replace("    CloseDetailModal,", "    CloseDetailModal,\n    PrintReturn(crate::models::sales_return::SalesReturn),\n    ClearPrint,")
        
        content = content[:enum_start] + enum_block + content[enum_end:]

    with open(filepath, "w") as f:
        f.write(content)
