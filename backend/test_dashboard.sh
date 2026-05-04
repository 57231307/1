#!/bin/bash
TOKEN=$(curl -s -X POST http://localhost:8082/api/v1/erp/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"password123"}' | jq -r '.data.token')
if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo "Login failed. Trying admin123"
  TOKEN=$(curl -s -X POST http://localhost:8082/api/v1/erp/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' | jq -r '.data.token')
fi
echo "Token: $TOKEN"

echo "Testing overview:"
curl -s http://localhost:8082/api/v1/erp/dashboard/overview?start_date=2026-05-01\&end_date=2026-05-03 -H "Authorization: Bearer $TOKEN" | jq

echo "Testing low-stock-alerts:"
curl -s http://localhost:8082/api/v1/erp/dashboard/low-stock-alerts -H "Authorization: Bearer $TOKEN" | jq

echo "Testing sales-stats:"
curl -s http://localhost:8082/api/v1/erp/dashboard/sales-stats?start_date=2026-01-01\&end_date=2026-03-31 -H "Authorization: Bearer $TOKEN" | jq

echo "Testing inventory-stats:"
curl -s http://localhost:8082/api/v1/erp/dashboard/inventory-stats -H "Authorization: Bearer $TOKEN" | jq

echo "Testing audit/stats:"
curl -s http://localhost:8082/api/v1/audit/stats -H "Authorization: Bearer $TOKEN" | jq

