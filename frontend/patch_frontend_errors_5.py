with open("src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

show_modal_logic = """
            Msg::ShowModalWithData(mode, inspection) => {
                self.modal_mode = mode;
                self.selected_inspection = Some(inspection);
                self.show_modal = true;
                true
            }"""

content = content.replace("            Msg::CloseModal => {", show_modal_logic + "\n            Msg::CloseModal => {")

with open("src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)
