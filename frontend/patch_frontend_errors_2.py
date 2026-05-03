import re

with open("src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

content = content.replace(
"""                            link.send_message(Msg::CompleteInspection(inspection));""",
"""                            link.send_message(Msg::ShowModalWithData(ModalMode::Complete, inspection));"""
)

# Replace the wrong ShowModalWithData implementation in complete modal view.
# Currently the ShowModalWithData sets `self.selected_inspection`.
# Msg::CompleteInspection receives a CompleteInspectionRequest and sends it to the service.
with open("src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)

with open("src/pages/greige_fabric.rs", "r") as f:
    content = f.read()

content = content.replace(
"""                    match GreigeFabricService::stock_out(id, crate::models::greige_fabric::StockOutRequest {
                        operator_id: 1, // default
                        reason: Some("手动出库".to_string()),
                    }).await {""",
"""                    match GreigeFabricService::stock_out(id, crate::models::greige_fabric::StockOutRequest {
                        weight_kg: None,
                        length_m: None,
                        remarks: Some("手动出库".to_string()),
                    }).await {"""
)
with open("src/pages/greige_fabric.rs", "w") as f:
    f.write(content)

