import subprocess
try:
    res = subprocess.check_output("psql -U postgres -d bingxi_erp -c 'SELECT * FROM roles;'", shell=True)
    print(res.decode('utf-8'))
except:
    print("Could not query")
