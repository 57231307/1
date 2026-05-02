import re

with open('src/main.rs', 'r') as f:
    content = f.read()

old_fallback = '''            let cookie_secret = settings.auth.cookie_secret.clone().unwrap_or_else(|| {
                tracing::warn!("警告: 未配置 auth.cookie_secret，系统正在使用降级的 jwt_secret 作为替代（这存在安全风险）");
                settings.auth.jwt_secret.clone()
            });'''

new_fallback = '''            let cookie_secret = settings.auth.cookie_secret.clone().unwrap_or_else(|| {
                tracing::warn!("警告: 未配置 auth.cookie_secret，系统正在使用降级的 jwt_secret 作为替代（这存在安全风险）");
                settings.auth.jwt_secret.clone()
            });
            
            if cookie_secret.len() < 32 {
                tracing::error!("严重安全配置错误: 用于 Cookie 加密的密钥 (cookie_secret 或降级的 jwt_secret) 长度不足 32 字节。请在 config.yaml 中配置至少 32 字节的强密钥！");
                std::process::exit(1);
            }'''

content = content.replace(old_fallback, new_fallback)

with open('src/main.rs', 'w') as f:
    f.write(content)
