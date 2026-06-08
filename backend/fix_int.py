import re
content = open('src/services/customer_credit_service.rs').read()

# total_amount >= 1000000
content = re.sub(r'total_amount >= (\d+)', r'total_amount >= rust_decimal::Decimal::from(\1)', content)

open('src/services/customer_credit_service.rs', 'w').write(content)
