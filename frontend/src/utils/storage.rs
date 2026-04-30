use web_sys::Window;

pub struct Storage;

impl Storage {
    fn get_window() -> Window {
        web_sys::window().expect("无法获取 window 对象")
    }

    fn get_local_storage() -> web_sys::Storage {
        let window = Self::get_window();
        window
            .local_storage()
            .expect("无法访问 localStorage")
            .expect("localStorage 不可用")
    }

    pub fn set_token(token: &str) {
        let storage = Self::get_local_storage();
        storage
            .set_item("auth_token", token)
            .expect("无法存储 token");
    }

    pub fn get_token() -> Option<String> {
        let storage = Self::get_local_storage();
        storage.get_item("auth_token").expect("无法读取 token")
    }

    pub fn remove_token() {
        let storage = Self::get_local_storage();
        storage.remove_item("auth_token").expect("无法删除 token");
    }

    #[allow(dead_code)]
    pub fn set_user_info(user_info: &str) {
        let storage = Self::get_local_storage();
        storage
            .set_item("user_info", user_info)
            .expect("无法存储用户信息");
    }

    #[allow(dead_code)]
    pub fn get_user_info() -> Option<String> {
        let storage = Self::get_local_storage();
        storage.get_item("user_info").expect("无法读取用户信息")
    }

    pub fn remove_user_info() {
        let storage = Self::get_local_storage();
        storage.remove_item("user_info").expect("无法删除用户信息");
    }

    pub fn clear_all() {
        Self::remove_token();
        Self::remove_user_info();
    }
}
