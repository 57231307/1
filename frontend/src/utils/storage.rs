use web_sys::Window;

pub struct Storage;

impl Storage {
    fn get_window() -> Option<Window> {
        web_sys::window()
    }

    fn get_local_storage() -> Option<web_sys::Storage> {
        Self::get_window()?.local_storage().ok().flatten()
    }

    pub fn set_token(token: &str) {
        if let Some(storage) = Self::get_local_storage() {
            let _ = storage.set_item("auth_token", token);
        }
    }

    pub fn get_token() -> Option<String> {
        Self::get_local_storage()?.get_item("auth_token").ok().flatten()
    }

    pub fn remove_token() {
        if let Some(storage) = Self::get_local_storage() {
            let _ = storage.remove_item("auth_token");
        }
    }

    #[allow(dead_code)]
    pub fn set_user_info(user_info: &str) {
        if let Some(storage) = Self::get_local_storage() {
            let _ = storage.set_item("user_info", user_info);
        }
    }

    #[allow(dead_code)]
    pub fn get_user_info() -> Option<String> {
        Self::get_local_storage()?.get_item("user_info").ok().flatten()
    }

    pub fn remove_user_info() {
        if let Some(storage) = Self::get_local_storage() {
            let _ = storage.remove_item("user_info");
        }
    }

    pub fn clear_all() {
        Self::remove_token();
        Self::remove_user_info();
    }
}
