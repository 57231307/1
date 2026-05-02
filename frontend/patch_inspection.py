import re
with open("/home/root0/桌面/121/1/frontend/src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

content = content.replace(
"""                        Ok(_inspection) => {
                            link.send_message(Msg::LoadError("功能开发中".to_string()));
                        }""",
"""                        Ok(inspection) => {
                            link.send_message(Msg::ShowModalWithData(ModalMode::Complete, inspection));
                        }"""
)
with open("/home/root0/桌面/121/1/frontend/src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)
