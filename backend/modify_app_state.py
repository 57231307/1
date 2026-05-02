import re

with open('src/utils/app_state.rs', 'r') as f:
    content = f.read()

# Update with_secrets to assert lengths
old_with_secrets = '''    pub fn with_secrets(db: Arc<DatabaseConnection>, jwt_secret: String, previous_jwt_secret: Option<String>, cookie_secret: String) -> Self {
        let metrics = MetricsService::new().expect("Failed to create metrics service");
        Self {'''

new_with_secrets = '''    pub fn with_secrets(db: Arc<DatabaseConnection>, jwt_secret: String, previous_jwt_secret: Option<String>, cookie_secret: String) -> Self {
        if cookie_secret.len() < 32 {
            tracing::error!("配置错误: cookie_secret 的长度必须至少为 32 字节。当前长度: {}", cookie_secret.len());
            std::process::exit(1);
        }
        
        let metrics = MetricsService::new().expect("Failed to create metrics service");
        Self {'''

content = content.replace(old_with_secrets, new_with_secrets)

with open('src/utils/app_state.rs', 'w') as f:
    f.write(content)
