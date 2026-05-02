import os
import re

filepath = 'src/pages/user_list.rs'
with open(filepath, 'r') as f:
    content = f.read()

# Remove user_service from struct
content = re.sub(r'\s*user_service:\s*UserService,', '', content)
content = re.sub(r'\s*user_service:\s*UserService::new\(\),', '', content)

# Change list_users call
# old: let service = self.user_service.clone();
#      ... service.list_users(page, page_size).await
# new: UserService::list_with_query(&crate::services::user_service::UserQuery { page, page_size }).await
content = re.sub(r'let service = self.user_service.clone\(\);\n', '', content)
content = re.sub(r'service\.list_users\(page, page_size\)', 'UserService::list_with_query(&crate::services::user_service::UserQuery { page, page_size })', content)

with open(filepath, 'w') as f:
    f.write(content)
