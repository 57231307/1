with open("/home/root0/桌面/121/1/frontend/src/pages/login.rs", "r") as f:
    content = f.read()

# 1. Update Msg::LoginSuccess
content = content.replace("LoginSuccess(String),", "LoginSuccess(crate::models::auth::LoginResponse),")

# 2. Update update match block
content = content.replace("link.send_message(Msg::LoginSuccess(response.token));", "link.send_message(Msg::LoginSuccess(response));")

# 3. Update Msg::LoginSuccess match arm
old_success = """            Msg::LoginSuccess(token) => {
                self.is_loading = false;
                Storage::set_token(&token);"""

new_success = """            Msg::LoginSuccess(resp) => {
                self.is_loading = false;
                Storage::set_token(&resp.token);
                if let Some(perms) = resp.permissions {
                    if let Ok(json) = serde_json::to_string(&perms) {
                        Storage::set_item("user_permissions", &json);
                    }
                }"""
content = content.replace(old_success, new_success)

with open("/home/root0/桌面/121/1/frontend/src/pages/login.rs", "w") as f:
    f.write(content)
