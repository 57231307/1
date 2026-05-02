import re

with open('backend/src/routes/mod.rs', 'r') as f:
    content = f.read()

# Add barcode_scanner_handler to the top imports
content = content.replace('    ap_verification_handler,', '    ap_verification_handler,\n    barcode_scanner_handler,')

# Add the route mapping
scanner_routes = '''    // 扫码出库路由
    let scanner_routes = Router::new()
        .route("/scan-to-ship", post(barcode_scanner_handler::scan_to_ship));

    // 物流管理路由'''

content = content.replace('    // 物流管理路由', scanner_routes)

nest_routes = '''        .nest("/api/v1/erp/logistics", logistics_routes)
        .nest("/api/v1/erp/scanner", scanner_routes)'''

content = content.replace('        .nest("/api/v1/erp/logistics", logistics_routes)', nest_routes)

with open('backend/src/routes/mod.rs', 'w') as f:
    f.write(content)
