# 🌍 contract-deployer 🛠️

Welcome to contract-deployer, a utility designed to deploy world-id-contracts and their dependencies to your chosen blockchain network.

## 📚 Prerequisites

Before you begin, ensure you have the following:

- Rust programming language installed on your system.
- Access to the target blockchain network.

To fetch all the submodules (world-id-contracts and its dependencies), run the following command:

```
> git submodule update --init --recursive
```

## ⚙️ Configuration

Configuration is a breeze with environment variables or command line arguments. Simply set these variables in a `.env` file located in the root directory of the project. We have provided a `.env.example` file to help you get started. Here's a sample `.env` file:

```
RUST_LOG=info
CONFIG=./polygon_mumbai_2023-05-25.yml
DEPLOYMENT_NAME=polygon_mumbai_2023-05-25
PRIVATE_KEY="3f96313dc0ccddd164bfa6b1396ee8975dfbb7ae3f38e12d98aae2c12cd32d8c"
RPC_URL="https://rpc.ankr.com/polygon_mumbai"
```

Here's a quick rundown of each variable:

- `RUST_LOG`: Sets the log level for the Rust application. Choose from `info`, `debug`, `warn`, `error`. It is recommended to set it to `info`.
- `CONFIG`: Specifies the path to the deployment configuration file.
- `DEPLOYMENT_NAME`: Names the deployment. Make it meaningful, like 'prod-2023-04-18'.
- `PRIVATE_KEY`: The private key used for the deployment.
- `RPC_URL`: The RPC Url used for the deployment.

Optional variables:

- `ETHERSCAN_API_KEY`: Your etherscan API key.
- `CACHE_DIR`: Cache directory. Default is `.cache`.

## 🚀 Usage

To launch the tool, use the following command:

```
cargo run
```

You can also pass options directly to the command line. Use `-h` or `--help` to see a summary of available options:

```
cargo run -- -h
```

Happy deploying! 🎉