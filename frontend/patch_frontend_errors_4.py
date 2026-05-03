with open("src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

content = content.replace("    CloseModal,\n    /// 创建检验单", "    CloseModal,\n    ShowModalWithData(ModalMode, PurchaseInspection),\n    /// 创建检验单")

with open("src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)
