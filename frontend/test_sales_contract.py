import requests
import json

base_url = 'http://127.0.0.1:8082/api/v1'

resp = requests.post(f"{base_url}/erp/auth/login", json={
    "username": "admin",
    "password": "password123"
}, headers={"X-Requested-With": "XMLHttpRequest"})
print("Login status:", resp.status_code)
print("Login text:", resp.text)
if resp.status_code == 200:
    data = resp.json()
    if 'token' in data:
        token = data['token']
    else:
        token = data['data']['token']
    print("Got token")
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
        "X-Requested-With": "XMLHttpRequest"
    }
    resp = requests.post(f"{base_url}/erp/sales-contracts", json={
        "contract_no": "SC-2026-006",
        "contract_name": "Test",
        "customer_id": 1,
        "total_amount": "5000.00",
        "payment_terms": "50%",
        "delivery_date": "2026-05-30"
    }, headers=headers)
    print("Contract POST status:", resp.status_code)
    print("Contract POST text:", resp.text)
