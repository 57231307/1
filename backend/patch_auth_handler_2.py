with open("/home/root0/桌面/121/1/backend/src/handlers/auth_handler.rs", "r") as f:
    content = f.read()

old_logic = """            let user_info = UserInfo {
                id: user.id,
                username: user.username.clone(),
                email: user.email.clone(),
                role_id: user.role_id,
            };

            let response = LoginResponse {
                token: token.clone(),
                user: user_info,
            };"""

new_logic = """            let mut permissions = vec![];
            if let Some(role_id) = user.role_id {
                let role_perms = crate::models::role_permission::Entity::find()
                    .filter(crate::models::role_permission::Column::RoleId.eq(role_id))
                    .filter(crate::models::role_permission::Column::Allowed.eq(true))
                    .all(state.db.as_ref())
                    .await
                    .unwrap_or_default();
                    
                permissions = role_perms.into_iter().map(|p| UserPermissionDto {
                    resource: p.resource_type,
                    action: p.action,
                    resource_id: p.resource_id,
                }).collect();
            }

            let user_info = UserInfo {
                id: user.id,
                username: user.username.clone(),
                email: user.email.clone(),
                role_id: user.role_id,
            };

            let response = LoginResponse {
                token: token.clone(),
                user: user_info,
                permissions,
            };"""

content = content.replace(old_logic, new_logic)

with open("/home/root0/桌面/121/1/backend/src/handlers/auth_handler.rs", "w") as f:
    f.write(content)
