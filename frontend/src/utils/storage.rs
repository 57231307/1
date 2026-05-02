use web_sys::Window;

pub struct Storage;

#[deprecated(since = "2.0.0", note = "请使用 HttpOnly Cookie，不再使用 localStorage")]
impl Storage {
    fn get_window() -> Option<Window> {
        web_sys::window()
    }

    fn get_session_storage() -> Option<web_sys::Storage> {
        Self::get_window()?.session_storage().ok().flatten()
    }

    pub fn set_token(_token: &str) {
        tracing::warn!("set_token 已废弃，请使用 HttpOnly Cookie");
        // 为了维持前端 SPA 路由守卫的逻辑，临时在 sessionStorage 存一个登录标记
        if let Some(storage) = Self::get_session_storage() {
            let _ = storage.set_item("is_authenticated", "true");
        }
    }

    pub fn get_token() -> Option<String> {
        tracing::warn!("get_token 已废弃，请使用 HttpOnly Cookie");
        // 返回一个 dummy 字符串欺骗路由守卫，如果 is_authenticated 存在
        if let Some(storage) = Self::get_session_storage() {
            if storage.get_item("is_authenticated").ok().flatten().is_some() {
                return Some("cookie_auth_active".to_string());
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
