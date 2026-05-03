with open("src/utils/storage.rs", "r") as f:
    content = f.read()

content = content.replace("    pub fn get_item", "impl Storage {\n    pub fn get_item")
content = content.replace("            }\n        }\n    }\n\n", "            }\n        }\n    }\n}\n\n")

with open("src/utils/storage.rs", "w") as f:
    f.write(content)
