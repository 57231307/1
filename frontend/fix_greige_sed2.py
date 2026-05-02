with open("src/pages/greige_fabric.rs", "r") as f:
    content = f.read()

content = content.replace("            quality_grade: None,\n            \n            remarks: None,", "            quality_grade: None,\n            purchase_date: None,\n            remarks: None,")
content = content.replace("                                    quality_grade: None,\n                                    \n                                    remarks: None,", "                                    quality_grade: None,\n                                    purchase_date: None,\n                                    remarks: None,")

with open("src/pages/greige_fabric.rs", "w") as f:
    f.write(content)
