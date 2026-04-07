import re
import json

def parse_warnings(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    structs = set(re.findall(r"warning: struct `([^`]+)` is never constructed", content))
    functions = set(re.findall(r"warning: function `([^`]+)` is never used", content))
    methods = set(re.findall(r"warning: method `([^`]+)` is never used", content))
    enums = set(re.findall(r"warning: enum `([^`]+)` is never used", content))
    unused_vars = set(re.findall(r"warning: unused variable: `([^`]+)`", content))
    unused_imports = set(re.findall(r"warning: unused import: `([^`]+)`", content))

    return {
        "structs": sorted(list(structs)),
        "functions": sorted(list(functions)),
        "methods": sorted(list(methods)),
        "enums": sorted(list(enums)),
        "vars": sorted(list(unused_vars)),
        "imports": sorted(list(unused_imports))
    }

backend = parse_warnings('/workspace/backend/backend_warnings.txt')
frontend = parse_warnings('/workspace/frontend/frontend_warnings.txt')

print("=== 后端未使用的模型 (Structs) ===")
for s in backend["structs"]: print(f"- {s}")
print("\n=== 后端未使用的函数/方法 (Functions/Methods) ===")
for f in backend["functions"]: print(f"- {f}()")
for m in backend["methods"]: print(f"- {m}()")
print("\n=== 后端未使用的引入 (Imports) ===")
for i in backend["imports"]: print(f"- {i}")

print("\n=== 前端未使用的模型 (Structs) ===")
for s in frontend["structs"]: print(f"- {s}")
print("\n=== 前端未使用的函数/方法 (Functions/Methods) ===")
for f in frontend["functions"]: print(f"- {f}()")
for m in frontend["methods"]: print(f"- {m}()")
print("\n=== 前端未使用的引入 (Imports) ===")
for i in frontend["imports"]: print(f"- {i}")
