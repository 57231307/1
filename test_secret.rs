fn main() {
    let secret = "change-me-to-a-secure-secret-in-production-at-least-32-bytes";
    let weak_patterns = [
        "change-in-production",
        "change-this",
        "local-dev",
        "your_secure",
        "default",
        "test",
        "example",
    ];
    for pattern in &weak_patterns {
        if secret.to_lowercase().contains(pattern) {
            println!("Contains weak pattern: {}", pattern);
            return;
        }
    }
    let unique_chars: std::collections::HashSet<char> = secret.chars().collect();
    let entropy_ratio = unique_chars.len() as f64 / secret.len() as f64;
    println!("Entropy: {}", entropy_ratio);
}
