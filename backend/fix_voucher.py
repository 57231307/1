import re

with open('src/services/voucher_service.rs', 'r') as f:
    content = f.read()

# Add import if missing
if 'use crate::utils::number_generator::DocumentNumberGenerator;' not in content:
    content = content.replace('use crate::models::voucher::', 'use crate::utils::number_generator::DocumentNumberGenerator;\nuse crate::models::voucher::')

# Replace the generate_voucher_no method
pattern = r'fn generate_voucher_no\([^)]*\)\s*->\s*Result<String,\s*AppError>\s*\{.*?\n    \}'

def replacer(m):
    return """async fn generate_voucher_no(
        &self,
        voucher_type: &str,
        _voucher_date: chrono::NaiveDate,
    ) -> Result<String, AppError> {
        let prefix = match voucher_type {
            "记" => "JZ",
            "收" => "SK",
            "付" => "FK",
            "转" => "ZZ",
            _ => "JZ",
        };

        DocumentNumberGenerator::generate_no(
            &*self.db,
            prefix,
            voucher::Entity,
            voucher::Column::VoucherNo,
        ).await
    }"""

content = re.sub(pattern, replacer, content, flags=re.DOTALL)

with open('src/services/voucher_service.rs', 'w') as f:
    f.write(content)
