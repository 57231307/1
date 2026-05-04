import requests

base_url = 'http://127.0.0.1:8082/api/v1'

resp = requests.post(f"{base_url}/erp/auth/login", json={
    "username": "admin",
    "password": "password123"
}, headers={"X-Requested-With": "XMLHttpRequest"})
token = resp.json()['data']['token']

resp = requests.get(f"{base_url}/erp/sales-contracts?page=1&page_size=50", headers={
    "Authorization": f"Bearer {token}",
    "X-Requested-With": "XMLHttpRequest"
})
data = resp.json()
for c in data.get('data', []):
    print(c['contract_no'], c['contract_name'], c['status'])
