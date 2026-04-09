from passlib.hash import argon2
hash_str = argon2.hash("password123")
print(hash_str)