#!/bin/bash
TOKEN=$(curl -s -X POST http://localhost:8083/api/v1/erp/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"password123"}' | jq -r '.data.token')
if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  TOKEN=$(curl -s -X POST http://localhost:8083/api/v1/erp/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' | jq -r '.data.token')
fi

echo "Token: $TOKEN"

curl -s http://localhost:8083/api/v1/erp/dashboard/overview?start_date=2026-05-01\&end_date=2026-05-03 -H "Authorization: Bearer $TOKEN" | jq
curl -s http://localhost:8083/api/v1/erp/dashboard/low-stock-alerts -H "Authorization: Bearer $TOKEN" | jq
curl -s http://localhost:8083/api/v1/erp/dashboard/sales-stats?start_date=2026-01-01\&end_date=2026-03-31 -H "Authorization: Bearer $TOKEN" | jq
curl -s http://localhost:8083/api/v1/erp/dashboard/inventory-stats -H "Authorization: Bearer $TOKEN" | jq
