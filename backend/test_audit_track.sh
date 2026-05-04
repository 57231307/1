#!/bin/bash
TOKEN=$(curl -s -X POST http://localhost:8082/api/v1/erp/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"password123"}' | jq -r '.data.token')
if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  TOKEN=$(curl -s -X POST http://localhost:8082/api/v1/erp/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' | jq -r '.data.token')
fi

curl -v -X POST http://localhost:8082/api/v1/audit/track \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"event_type": "UI_CLICK", "module": "dashboard", "action": "view", "details": {"path": "/dashboard"}}'
