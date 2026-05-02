import re

with open('backend/src/handlers/finance_payment_handler.rs', 'r') as f:
    content = f.read()

header = '''//! 通用财务支付网关 Handler
//! 
//! 区别于 ap_payment (仅限应付账款)，此模块提供全局统一的财务支付入口
//! 能够接收来自采购、销售、退货、人工调整等所有渠道的支付动作。
'''

if '通用财务支付网关' not in content:
    content = header + '\n' + content

with open('backend/src/handlers/finance_payment_handler.rs', 'w') as f:
    f.write(content)
