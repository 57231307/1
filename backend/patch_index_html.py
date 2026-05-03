with open("/home/root0/桌面/121/1/frontend/index.html", "r") as f:
    content = f.read()

script_tag = '<script src="https://cdnjs.cloudflare.com/ajax/libs/html2pdf.js/0.10.1/html2pdf.bundle.min.js"></script>'
if script_tag not in content:
    content = content.replace("</head>", f"    {script_tag}\n</head>")

with open("/home/root0/桌面/121/1/frontend/index.html", "w") as f:
    f.write(content)
