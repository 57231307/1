import re

with open('src/services/totp_service.rs', 'r') as f:
    content = f.read()

# Replace the first occurrence
old_check_1 = "if totp.check_current(token).unwrap_or(false)  {"
new_check_1 = '''let is_valid = match totp.check_current(token) {
            Ok(valid) => valid,
            Err(e) => {
                tracing::warn!("TOTP 验证内部发生异常: {}", e);
                false
            }
        };

        if is_valid {'''
content = content.replace(old_check_1, new_check_1)

# Replace the second occurrence
old_check_2 = "Ok(totp.check_current(token).unwrap_or(false) )"
new_check_2 = '''let is_valid = match totp.check_current(token) {
            Ok(valid) => valid,
            Err(e) => {
                tracing::warn!("TOTP 验证内部发生异常: {}", e);
                false
            }
        };
        Ok(is_valid)'''
content = content.replace(old_check_2, new_check_2)

with open('src/services/totp_service.rs', 'w') as f:
    f.write(content)
