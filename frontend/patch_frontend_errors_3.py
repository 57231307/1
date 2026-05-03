with open("src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

# Add ShowModalWithData to Msg enum
if "ShowModalWithData(ModalMode, PurchaseInspection)" not in content:
    content = content.replace("    CloseModal,\n}", "    CloseModal,\n    ShowModalWithData(ModalMode, PurchaseInspection),\n}")

# Add handling in update
if "Msg::ShowModalWithData" not in content:
    show_modal_logic = """
            Msg::ShowModalWithData(mode, inspection) => {
                self.modal_mode = mode;
                self.selected_inspection = Some(inspection);
                self.show_modal = true;
                true
            }"""
    # put it before Msg::CloseModal
    content = content.replace("            Msg::CloseModal => {", show_modal_logic + "\n            Msg::CloseModal => {")

with open("src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)
