with open("src/pages/greige_fabric.rs", "r") as f:
    content = f.read()

content = content.replace(
"""            quality_grade: None,
            remarks: None,
            created_by: None,
            created_at: String::new(),""",
"""            quality_grade: None,
            purchase_date: None,
            remarks: None,
            created_by: None,
            created_at: String::new(),"""
)

content = content.replace(
"""                                    quality_grade: None,
                                    remarks: None,
                                    created_by: None,""",
"""                                    quality_grade: None,
                                    purchase_date: None,
                                    remarks: None,
                                    created_by: None,"""
)

with open("src/pages/greige_fabric.rs", "w") as f:
    f.write(content)
