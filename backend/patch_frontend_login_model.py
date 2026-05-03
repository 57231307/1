with open("/home/root0/桌面/121/1/frontend/src/models/auth.rs", "r") as f:
    content = f.read()

content = content.replace("    pub user: UserInfo,\n}", "    pub user: UserInfo,\n    pub permissions: Option<Vec<crate::utils::permissions::UserPermission>>,\n}")

with open("/home/root0/桌面/121/1/frontend/src/models/auth.rs", "w") as f:
    f.write(content)
