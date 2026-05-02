with open("src/pages/fabric_order.rs", "r") as f:
    content = f.read()

content = content.replace(
"""                    let req = UpdateFabricOrderRequest {
                        customer_id: Some(get_val("customer-id").parse().unwrap_or(0)),
                        order_date: Some(get_val("order-date")),
                        required_date: Some(get_val("required-date")),
                        status: None,
                        shipping_address: None,
                        delivery_address: None,
                        payment_terms: None,
                        remarks: get_opt("remarks"),
                        batch_no: get_opt("batch-no"),
                        color_no: get_opt("color-no"),
                        dye_lot_no: None,
                        grade: None,
                        packaging_requirement: None,
                        quality_standard: None,
                    };""",
"""                    let req = UpdateFabricOrderRequest {
                        required_date: Some(get_val("required-date")),
                        status: None,
                        shipping_address: None,
                        delivery_address: None,
                        payment_terms: None,
                        remarks: get_opt("remarks"),
                        items: None,
                        batch_no: get_opt("batch-no"),
                        color_no: get_opt("color-no"),
                        packaging_requirement: None,
                        quality_standard: None,
                    };"""
)

# Fix order_clone move
content = content.replace("let order_clone = order.clone();", "let order_clone = order.clone();\n                            let order_clone2 = order.clone();")
content = content.replace("Msg::OpenModal(ModalMode::Edit, Some(order_clone.clone()))", "Msg::OpenModal(ModalMode::Edit, Some(order_clone2))")

with open("src/pages/fabric_order.rs", "w") as f:
    f.write(content)
