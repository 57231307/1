import requests
import json

response = requests.post("http://127.0.0.1:8082/api/v1/auth/login", json={
    "username": "admin",
    "password": "password123"
})
token = response.json()["data"]["token"]

res = requests.get("http://127.0.0.1:8082/api/v1/erp/sales-contracts", headers={"Authorization": f"Bearer {token}"})
print(json.dumps(res.json(), indent=2))
