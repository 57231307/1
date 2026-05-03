for filepath in ["src/pages/ap_invoice.rs", "src/pages/ar_invoice.rs"]:
    with open(filepath, "r") as f:
        content = f.read()
    
    enum_start = content.find("pub enum Msg {")
    enum_end = content.find("}", enum_start)
    enum_block = content[enum_start:enum_end]
    
    item_type = "ApInvoice" if "ap_invoice" in filepath else "ArInvoice"
    
    if f"PrintInvoice(crate::models" not in enum_block and "PrintInvoice" not in enum_block:
        # Just replace ChangePage(u64), with ChangePage(u64), PrintInvoice(crate::models::..), ClearPrint,
        if "ChangePage(u64)," in enum_block:
            enum_block = enum_block.replace("ChangePage(u64),", f"ChangePage(u64),\n    PrintInvoice(crate::models::{item_type.lower()}::{item_type}),\n    ClearPrint,")
        
    content = content[:enum_start] + enum_block + content[enum_end:]
    with open(filepath, "w") as f:
        f.write(content)
