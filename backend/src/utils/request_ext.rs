/// 请求扩展，用于缓存公共路径检查结果
#[derive(Clone, Debug)]
pub struct PublicPathCache {
    pub is_public: bool,
}

impl PublicPathCache {
    pub fn new(is_public: bool) -> Self {
        Self { is_public }
    }
}
