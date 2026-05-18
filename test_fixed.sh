#!/bin/bash
SERVER_IP="111.230.99.236"
BASE_URL="http://${SERVER_IP}/api/v1/erp"
TOKEN=$(curl -s -X POST "${BASE_URL}/auth/login" -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' | jq -r '.data.token')

echo "=== 验证 500 错误修复 ==="
echo "1. /warehouses/locations (修复 locations 表名):"
curl -s "${BASE_URL}/warehouses/locations" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'

echo "2. /bpm/monitor/stats (修复 applicant_id):"
curl -s "${BASE_URL}/bpm/monitor/stats" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'

echo "3. /ap/reconciliations:"
curl -s "${BASE_URL}/ap/reconciliations" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'

echo "4. /budgets/plans:"
curl -s "${BASE_URL}/budgets/plans" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'

echo ""
echo "=== 验证 405 方法修复 ==="
echo "5. POST /dual-unit/convert:"
curl -s -X POST "${BASE_URL}/dual-unit/convert" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"quantity":1,"from_unit":"kg","to_unit":"g"}' | jq -r '.code // "success"'

echo "6. POST /bpm/process/start:"
curl -s -X POST "${BASE_URL}/bpm/process/start" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"process_definition_id":1,"business_type":"test","business_id":1,"initiator_id":1,"variables":{}}' | jq -r '.code // "success"'

echo ""
echo "=== 验证 400 参数修复 ==="
echo "7. /customers/1/summary:"
curl -s "${BASE_URL}/customers/1/summary" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'

echo "8. /ap/reports/statistics (带参数):"
curl -s "${BASE_URL}/ap/reports/statistics?start_date=2026-01-01&end_date=2026-12-31" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'

echo "9. /finance/accounting-periods/current (正确路径):"
curl -s "${BASE_URL}/finance/accounting-periods/current" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'

echo "10. /bpm/tasks (带参数):"
curl -s "${BASE_URL}/bpm/tasks?status=PENDING" -H "Authorization: Bearer $TOKEN" | jq -r '.code // "success"'
