with open("/home/root0/桌面/121/1/frontend/src/pages/purchase_order.rs", "r") as f:
    content = f.read()

# 1. Add ExportPdf Msg
if "ExportPdf(crate::models::purchase_order::PurchaseOrder)," not in content:
    content = content.replace("    ClearPrint,", "    ClearPrint,\n    ExportPdf(crate::models::purchase_order::PurchaseOrder),")

# 2. Add ExportPdf handling
export_logic = """
            Msg::ExportPdf(o) => {
                crate::utils::pdf_export::export_to_pdf("pdf-export-content", &format!("purchase_order_{}.pdf", o.order_no));
                true
            }"""
if "Msg::ExportPdf" not in content:
    content = content.replace("            Msg::ClearPrint => {", export_logic + "\n            Msg::ClearPrint => {")

# 3. Add Export PDF button
button_html = """<button class="btn-secondary" onclick={link.callback(move |_| Msg::ExportPdf(order.clone()))}>{"导出PDF"}</button>"""
if "Msg::ExportPdf(order" not in content:
    content = content.replace("""<button class="btn-info" onclick={link.callback(move |_| Msg::PrintOrder(order.clone()))}>{"打印"}</button>""", button_html + """\n                                <button class="btn-info" onclick={link.callback(move |_| Msg::PrintOrder(order.clone()))}>{"打印"}</button>""")

# 4. Add id to modal content for pdf export
if 'id="pdf-export-content"' not in content:
    content = content.replace('<div class="modal-content">', '<div class="modal-content" id="pdf-export-content">')

with open("/home/root0/桌面/121/1/frontend/src/pages/purchase_order.rs", "w") as f:
    f.write(content)
