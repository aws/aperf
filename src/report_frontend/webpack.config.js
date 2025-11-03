const HtmlWebpackPlugin = require("html-webpack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");

module.exports = (env) => {
  return {
    mode: "production",
    entry: ["./src/report_frontend/src/react-app.tsx"],
    resolve: {
      extensions: [".tsx", ".ts", ".js"],
      modules: [".", "node_modules"],
    },
    plugins: [
      new HtmlWebpackPlugin({
        template: "./src/report_frontend/src/index.html",
        favicon: "./src/report_frontend/public/favicon.ico",
      }),
      new MiniCssExtractPlugin(),
    ],
    module: {
      rules: [
        {
          test: /\.tsx?$/,
          use: "babel-loader",
          exclude: "/node_modules/",
        },
        {
          test: /\.css$/,
          use: [MiniCssExtractPlugin.loader, "css-loader"],
        },
        {
          test: /\.ico$/,
          loader: "file-loader",
        },
      ],
    },
    output: {
      filename: "bundle.js",
      path: env.output,
    },
  };
};
