const webpack = require('webpack');
const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const OptimizeCSSPlugin = require('optimize-css-assets-webpack-plugin');

const productionPlugins = [
  new OptimizeCSSPlugin({
    cssProcessorOptions: {
      safe: true,
      map: { inline: false }
    }
  })
];

module.exports = (env, argv) => ({
  entry: path.resolve(__dirname, 'client/index.ts'),
  output: {
    chunkFilename: '[name].js',
    filename: '[name].js',
    path: path.resolve(__dirname, 'static')
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        include: path.resolve(__dirname, 'ts'),
        loader: 'babel-loader'
      },
      {
        test: /\.sass$/,
        use: [
          MiniCssExtractPlugin.loader,
          'css-loader',
          'sass-loader?indentedSyntax'
        ]
      },
      {
        test: /\.png$|\.svg$|\.eot$|\.woff$|\.woff2$|\.ttf$/,
        use: 'file-loader'
      }
    ]
  },
  mode: argv.mode || 'development',
  devtool: 'source-map',
  plugins: [
    new webpack.DefinePlugin({
      'process.env': {
        'NODE_ENV': JSON.stringify(argv.mode || 'development')
      }
    }),
    new HtmlWebpackPlugin({
      filename: 'index.html'
    }),
    ...(argv.mode === 'production' ? productionPlugins : [])
  ],
  optimization: {
    splitChunks: {
      cacheGroups: {
        vendors: {
          priority: -10,
          test: /[\\/]node_modules[\\/]/
        }
      },
      chunks: 'async',
      minChunks: 1,
      minSize: 30000,
      name: true
    }
  },
  node: {
    setImmediate: false,
    dgram: 'empty',
    fs: 'empty',
    net: 'empty',
    tls: 'empty',
    child_process: 'empty'
  }
});
