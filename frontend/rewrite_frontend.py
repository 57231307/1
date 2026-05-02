import os
import re

SERVICES_DIR = '/home/root0/桌面/121/1/frontend/src/services'

def rewrite_service(filename, model_module, service_name, entity_name, create_req, update_req, list_res, query_type=None, base_path=None):
    filepath = os.path.join(SERVICES_DIR, filename)
    if not os.path.exists(filepath):
        return

    with open(filepath, 'r') as f:
        content = f.read()

    # Check if already implements CrudService
    if 'impl CrudService for' in content:
        print(f"Skipping {filename} - already using CrudService")
        return
        
    if not base_path:
        base_path = f"/{entity_name}s" # naive plural

    # Identify custom methods to preserve
    # We will just rewrite the whole file, but preserve methods that aren't standard CRUD
    # Standard methods: list, get, create, update, delete
    # It's easier to just replace the standard methods and add the impl CrudService
    
    # We'll just generate the top part and keep the rest as `impl ServiceName { ... }` 
    # but removing the standard methods.
    
    # regex to remove standard methods
    content = re.sub(rf'(?:\s*pub async fn list\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn list_{entity_name}s\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn get\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn get_{entity_name}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn create\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn create_{entity_name}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn update\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn update_{entity_name}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn delete\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    content = re.sub(rf'(?:\s*pub async fn delete_{entity_name}\b.*?\n(?:        .*?\n)*?    \}})', '', content, flags=re.MULTILINE)
    
    # Check if impl ServiceName is now empty
    content = re.sub(rf'impl {service_name} {{\s*}}', '', content)
    
    # Add imports
    if 'use crate::services::crud_service::CrudService;' not in content:
        content = content.replace('use crate::services::api::ApiService;', 'use crate::services::api::ApiService;\nuse crate::services::crud_service::CrudService;')
        if 'ApiService;' not in content:
            content = 'use crate::services::crud_service::CrudService;\n' + content

    # Prepend the CrudService implementation
    struct_def = f"pub struct {service_name};"
    impl_crud = f"""
impl CrudService for {service_name} {{
    type Model = {entity_name};
    type ListResponse = {list_res};
    type CreateRequest = {create_req};
    type UpdateRequest = {update_req};

    fn base_path() -> &'static str {{
        "{base_path}"
    }}
}}
"""
    content = content.replace(struct_def, struct_def + "\n" + impl_crud)

    with open(filepath, 'w') as f:
        f.write(content)
    print(f"Updated {filename}")

rewrite_service('customer_service.rs', 'customer', 'CustomerService', 'Customer', 'CreateCustomerRequest', 'UpdateCustomerRequest', 'CustomerListResponse')
rewrite_service('supplier_service.rs', 'supplier', 'SupplierService', 'Supplier', 'CreateSupplierRequest', 'UpdateSupplierRequest', 'crate::models::supplier::SupplierListResponse')
rewrite_service('product_service.rs', 'product', 'ProductService', 'Product', 'CreateProductRequest', 'UpdateProductRequest', 'ProductListResponse')
rewrite_service('role_service.rs', 'role', 'RoleService', 'Role', 'CreateRoleRequest', 'UpdateRoleRequest', 'RoleListResponse')
# User service uses different models perhaps
# rewrite_service('user_service.rs', 'user', 'UserService', 'UserResponse', 'CreateUserRequest', 'UpdateUserRequest', 'PaginatedResponse<UserResponse>', base_path="/users")
