from passlib.hash import argon2
hash = argon2.using(rounds=2, memory_cost=19456, parallelism=1).hash("password123")
print(hash)
