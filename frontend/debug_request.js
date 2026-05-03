fetch("http://127.0.0.1:8081/api/v1/erp/init/test-database", {
  method: "POST",
  headers: {"Content-Type": "application/json"},
  body: JSON.stringify({
    host: "39.99.34.194",
    port: "5432",
    name: "bingxi",
    username: "bingxi",
    password: "d5eb610ccf1a701dac02d5.dbcba8f5f546a"
  })
}).then(r => r.text()).then(console.log).catch(console.error);
