import os
import re
import glob

handlers_dir = 'backend/src/handlers/'
files = glob.glob(os.path.join(handlers_dir, '*.rs'))

# We want to find a handler like product_handler.rs or sales_order_handler.rs
# where we can mask data based on auth.role_id

content = """
use crate::middleware::auth_context::AuthContext;
use serde_json::Value;

pub fn mask_sensitive_fields(mut value: Value, auth: &AuthContext) -> Value {
    # If not admin (role_id != 1) mask cost_price and other sensitive fields
    if auth.role_id != 1 {
        if value.is_object() {
            let obj = value.as_object_mut().unwrap();
            if obj.contains_key("cost_price") {
                obj.remove("cost_price");
            }
            if obj.contains_key("price") {
                # Maybe mask it to 0 or remove
                obj.insert("price".to_string(), Value::String("***".to_string()));
            }
            # recursive masking could be applied
        } else if value.is_array() {
            let arr = value.as_array_mut().unwrap();
            for item in arr.iter_mut() {
                *item = mask_sensitive_fields(item.clone(), auth);
            }
        }
    }
    value
}
"""

print("We can create a field_mask utility in backend/src/utils/field_mask.rs")
