import os
import re
import glob

SERVICES_DIR = '/home/root0/桌面/121/1/frontend/src/services'

for filepath in glob.glob(os.path.join(SERVICES_DIR, "*.rs")):
    filename = os.path.basename(filepath)
    with open(filepath, 'r') as f:
        content = f.read()
    
    if 'impl CrudService for' in content:
        continue
    
    # Try to find standard methods
    list_match = re.search(r'pub async fn list\w*\s*\([^)]*\)\s*->\s*Result<([^,]+)', content)
    get_match = re.search(r'pub async fn get\w*\s*\([^)]*\)\s*->\s*Result<([^,]+)', content)
    create_match = re.search(r'pub async fn create\w*\s*\([^:]*:\s*&?([^)]+)\)\s*->\s*Result<([^,]+)', content)
    update_match = re.search(r'pub async fn update\w*\s*\([^,]*,[^:]*:\s*&?([^)]+)\)\s*->\s*Result<([^,]+)', content)
    
    if not (list_match and get_match and create_match and update_match):
        continue
        
    list_res = list_match.group(1).strip()
    model = get_match.group(1).strip()
    create_req = create_match.group(1).strip()
    update_req = update_match.group(1).strip()
    
    # Extract service name
    service_name_match = re.search(r'pub struct (\w+);', content)
    if not service_name_match:
        continue
    service_name = service_name_match.group(1)
    
    # Extract base path from one of the API calls
    base_path_match = re.search(r'ApiService::get[^"]*"([^"?]+)', content)
    if not base_path_match:
        continue
    base_path = base_path_match.group(1).split('/{}')[0].strip()
    
    print(f"{filename}: model={model}, list={list_res}, create={create_req}, update={update_req}, path={base_path}")
