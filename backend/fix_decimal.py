import sys, re

def fix_file(path):
    content = open(path).read()
    # Decimal::parse_bytes(X.to_string().as_bytes(), 10)
    # to X.to_string().parse::<Decimal>().unwrap()
    content = re.sub(r'Decimal::parse_bytes\((.*?)\.to_string\(\)\.as_bytes\(\),\s*10\)', r'\1.to_string().parse::<rust_decimal::Decimal>().unwrap()', content)
    
    # Decimal::parse_bytes(s.as_bytes(), 10)
    # to s.parse::<Decimal>()
    content = re.sub(r'Decimal::parse_bytes\((.*?)\.as_bytes\(\),\s*10\)', r'\1.parse::<rust_decimal::Decimal>()', content)
    
    open(path, 'w').write(content)

fix_file('src/services/so/order.rs')
fix_file('src/services/customer_credit_service.rs')
