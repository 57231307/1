use web_sys::Window;

pub struct Storage;

impl Storage {
    pub fn get_item(key: &str) -> Option<String> {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                return storage.get_item(key).ok().flatten();
            }
        }
        None
    }

    pub fn set_item(key: &str, value: &str) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(key, value);
            }
        }
    }
}


#[deprecated(since = "2.0.0", note = "请使用 HttpOnly Cookie，不再使用 localStorage")]
impl Storage {
    fn get_window() -> Option<Window> {
        web_sys::window()
    }

    fn get_session_storage() -> Option<web_sys::Storage> {
        Self::get_window()?.session_storage().ok().flatten()
    }

    pub fn set_token(token: &str) {
        // 为了维持前端 SPA 路由守卫的逻辑，在 sessionStorage 存储认证标记和真实token
        if let Some(storage) = Self::get_session_storage() {
            let _ = storage.set_item("is_authenticated", "true");
            let _ = storage.set_item("auth_token", token); // 存储真实token用于API请求
        }
    }

    pub fn get_token() -> Option<String> {
        // 从 sessionStorage 获取真实token
        if let Some(storage) = Self::get_session_storage() {
            // 先检查是否已认证
            if storage.get_item("is_authenticated").ok().flatten().is_some() {
                // 返回真实token（如果存在），否则返回dummy字符串
                return storage.get_item("auth_token").ok().flatten()
                    .or_else(|| Some("cookie_auth_active".to_string()));
            }
        }
        None
    }

    pub fn remove_token() {
        tracing::warn!("remove_token 已废弃，请使用 HttpOnly Cookie");
        if let Some(storage) = Self::get_session_storage() {
            let _ = storage.remove_item("is_authenticated");
        }
    }

    #[allow(dead_code)]
    pub fn set_user_info(user_info: &str) {
        if let Some(storage) = Self::get_session_storage() {
            let _ = storage.set_item("user_info", user_info);
        }
    }

    #[allow(dead_code)]
    pub fn get_user_info() -> Option<String> {
        Self::get_session_storage()?.get_item("user_info").ok().flatten()
    }

    pub fn remove_user_info() {
        if let Some(storage) = Self::get_session_storage() {
            let _ = storage.remove_item("user_info");
        }
    }

    pub fn clear_all() {
        Self::remove_token();
        Self::remove_user_info();
    }
}
