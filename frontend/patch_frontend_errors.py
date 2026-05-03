with open("src/pages/purchase_inspection.rs", "r") as f:
    content = f.read()

content = content.replace(
"""                            link.send_message(Msg::ShowModalWithData(ModalMode::Complete, inspection));""",
"""                            link.send_message(Msg::CompleteInspection(inspection));"""
)
with open("src/pages/purchase_inspection.rs", "w") as f:
    f.write(content)

with open("src/pages/greige_fabric.rs", "r") as f:
    content = f.read()

# Fix GreigeFabricService::stock_out(id).await missing req
content = content.replace(
"""                    match GreigeFabricService::stock_out(id).await {""",
"""                    match GreigeFabricService::stock_out(id, crate::models::greige_fabric::StockOutRequest {
                        operator_id: 1, // default
                        reason: Some("手动出库".to_string()),
                    }).await {"""
)
with open("src/pages/greige_fabric.rs", "w") as f:
    f.write(content)

