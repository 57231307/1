import re

file_path = "frontend/src/app/mod.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

broken_routes = [
    "QualityInspection", "FinancialAnalysis", "SupplierEvaluation"
]

for route in broken_routes:
    content = re.sub(r'#\[at\("[^"]+"\)]\s+' + route + r',?\s*', '', content)
    content = re.sub(r'Route::' + route + r'\s*=>\s*protected_route\(\|\| html! \{ <[^>]+> \}\),?\s*', '', content)

with open(file_path, "w", encoding="utf-8") as f:
    f.write(content)

print("Removed remaining broken routes")
