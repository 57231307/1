module.exports = {
  mode: "development",
  entry: [{
    import: "./src/main.rs",
    name: "main"
  }],
  output: {
    path: "./dist",
    publicPath: "/",
    filename: "[name].[contenthash:8].js"
  },
  devServer: {
    port: 8080,
    host: "0.0.0.0",
    hot: true,
    open: true
  }
};
