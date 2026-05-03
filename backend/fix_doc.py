with open("src/services/ap_payment_service.rs", "r") as f:
    lines = f.readlines()

if "use crate::services::event_bus" in lines[0]:
    import_line = lines[0]
    lines = lines[1:]
    
    # insert after the last //!
    insert_idx = 0
    for i, line in enumerate(lines):
        if line.startswith("//!") or line.startswith("#![allow"):
            insert_idx = i + 1
        else:
            break
            
    lines.insert(insert_idx, import_line)
    
    with open("src/services/ap_payment_service.rs", "w") as f:
        f.writelines(lines)
