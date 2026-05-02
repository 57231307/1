import re

with open('src/services/supplier_service.rs', 'r') as f:
    content = f.read()

# Add import if missing
if 'use crate::utils::number_generator::DocumentNumberGenerator;' not in content:
    content = content.replace('use crate::models::supplier::', 'use crate::utils::number_generator::DocumentNumberGenerator;\nuse crate::models::supplier::')

# Replace the generate_supplier_code method
pattern = r'pub async fn generate_supplier_code\(&self\) -> Result<String, AppError> \{.*?\n    \}'

def replacer(m):
    return """pub async fn generate_supplier_code(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "SUP",
            supplier::Entity,
            supplier::Column::SupplierCode,
        ).await
    }"""

content = re.sub(pattern, replacer, content, flags=re.DOTALL)

with open('src/services/supplier_service.rs', 'w') as f:
    f.write(content)
