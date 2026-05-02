sed -i 's/ApInvoiceService::list_invoices(/ApInvoiceService::list_with_query(\&/g' src/pages/ap_invoice.rs
sed -i 's/ApPaymentService::list_payments(/ApPaymentService::list_with_query(\&/g' src/pages/ap_payment.rs
sed -i 's/ApPaymentRequestService::list_requests(/ApPaymentRequestService::list_with_query(\&/g' src/pages/ap_payment_request.rs
sed -i 's/ArInvoiceService::list_invoices(/ArInvoiceService::list_with_query(\&/g' src/pages/ar_invoice.rs
sed -i 's/CustomerCreditService::list_credits(/CustomerCreditService::list_with_query(\&/g' src/pages/customer_credit.rs
sed -i 's/FinanceInvoiceService::list_invoices(/FinanceInvoiceService::list_with_query(\&/g' src/pages/finance_invoice.rs
sed -i 's/FinancePaymentService::list_payments(/FinancePaymentService::list_with_query(\&/g' src/pages/finance_payment.rs
sed -i 's/PurchaseContractService::list_contracts(/PurchaseContractService::list_with_query(\&/g' src/pages/purchase_contract.rs
sed -i 's/SalesContractService::list_contracts(/SalesContractService::list_with_query(\&/g' src/pages/sales_contract.rs
sed -i 's/SalesService::list_orders(/SalesService::list_with_query(\&/g' src/pages/sales_order.rs
sed -i 's/WarehouseService::list_warehouses()/WarehouseService::list()/g' src/pages/sales_order.rs

# Also for get, delete, create, update
sed -i 's/ApInvoiceService::delete_invoice(/ApInvoiceService::delete(/g' src/pages/ap_invoice.rs
sed -i 's/ApPaymentService::delete_payment(/ApPaymentService::delete(/g' src/pages/ap_payment.rs
sed -i 's/ApPaymentRequestService::delete_request(/ApPaymentRequestService::delete(/g' src/pages/ap_payment_request.rs
sed -i 's/ArInvoiceService::delete_invoice(/ArInvoiceService::delete(/g' src/pages/ar_invoice.rs
sed -i 's/FinanceInvoiceService::delete_invoice(/FinanceInvoiceService::delete(/g' src/pages/finance_invoice.rs
sed -i 's/FinancePaymentService::delete_payment(/FinancePaymentService::delete(/g' src/pages/finance_payment.rs

sed -i 's/PurchaseContractService::create_contract(/PurchaseContractService::create(/g' src/pages/purchase_contract.rs
sed -i 's/SalesContractService::create_contract(/SalesContractService::create(/g' src/pages/sales_contract.rs

sed -i 's/SalesService::get_order(/SalesService::get(/g' src/pages/sales_order.rs
