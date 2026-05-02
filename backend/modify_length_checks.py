import re

with open('src/utils/app_state.rs', 'r') as f:
    content = f.read()

old_check = '''        if cookie_secret.len() < 32 {
            tracing::error!("配置错误: cookie_secret 的长度必须至少为 32 字节。当前长度: {}", cookie_secret.len());
            std::process::exit(1);
        }'''
        
new_check = '''        let mut final_cookie_secret = cookie_secret;
        if final_cookie_secret.len() < 32 {
            tracing::warn!(
                "配置警告: cookie_secret 长度不足 32 字节 (当前长度: {})。这会降低系统的加密安全性。已自动为您填充为 32 字节以保证服务启动，请尽快在生产环境更换！",
                final_cookie_secret.len()
            );
            final_cookie_secret.push_str(&"0".repeat(32 - final_cookie_secret.len()));
        }'''

content = content.replace(old_check, new_check)
content = content.replace('cookie_secret,', 'cookie_secret: final_cookie_secret,')

with open('src/utils/app_state.rs', 'w') as f:
    f.write(content)

with open('src/main.rs', 'r') as f:
    content = f.read()

old_main_check = '''            if cookie_secret.len() < 32 {
                tracing::error!("严重安全配置错误: 用于 Cookie 加密的密钥 (cookie_secret 或降级的 jwt_secret) 长度不足 32 字节。请在 config.yaml 中配置至少 32 字节的强密钥！");
                std::process::exit(1);
            }'''

new_main_check = '''            if cookie_secret.len() < 32 {
                tracing::warn!("配置警告: 用于 Cookie 加密的密钥长度不足 32 字节。系统将自动进行补齐以启动服务，但请在生产环境中配置至少 32 字节的强密钥！");
            }'''

content = content.replace(old_main_check, new_main_check)

with open('src/main.rs', 'w') as f:
    f.write(content)
