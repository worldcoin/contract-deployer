# 🌍 contract-deployer 🛠️

Welcome to contract-deployer, a utility designed to deploy world-id-contracts and their dependencies to your chosen blockchain network.

## 📚 Prerequisites

Before you begin, ensure you have the following:

- Rust programming language installed on your system.
- Access to the target blockchain network.

To fetch all the submodules (world-id-contracts and its dependencies), run the following command:

```bash
git submodule update --init --recursive
```

## ⚙️ Configuration

Configuration is a breeze with environment variables or command line arguments. Simply set these variables in a `.env` file located in the root directory of the project. We have provided a `.env.example` file to help you get started. Here's a sample `.env` file:

```env
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

## Configuration file

The configuration file, specified via the `CONFIG` env var is structured YAML and contains two main sections: `groups` and `misc`.

An example configuration file looks like this:

```yaml
groups:
  1: # Orb
    tree_depth: 30
    batch_sizes:
      - 10
      - 100
      - 1000
  0: # Phone
    tree_depth: 30
    batch_sizes:
      - 100
misc:
  initial_leaf_value: "0x0000000000000000000000000000000000000000000000000000000000000000"
# world_id_contract_commit_hash: '2e2d25f1c45b07657e8830fb85a5221941aac68e'
```

Notes on values:
 - `tree_depth` is limited by [the Semaphore](https://github.com/worldcoin/semaphore-v3). Permissible [range 16-32](https://github.com/worldcoin/world-id-contracts/blob/master/src/utils/SemaphoreTreeDepthValidator.sol#L13-L14)

### Groups

The `groups` section is a map where each key is a `GroupId` and the value is a `GroupConfig`. Each `GroupConfig` has two properties: `tree_depth` and `batch_sizes`.

- `tree_depth` (TreeDepth): This is a numerical value representing the depth of the tree for the group. In the provided example, both groups have a tree depth of 30.

- `batch_sizes` (Vec<BatchSize>): This is a list of batch sizes for the group. In the provided example, the group with `GroupId` 1 has batch sizes of 10, 100, and 1000, while the group with `GroupId` 0 has a batch size of 100.

### Misc

The `misc` section contains miscellaneous configuration options. Currently, it only contains one property: `initial_leaf_value`.

- `initial_leaf_value` (H256): This is a hexadecimal value representing the initial leaf value. In the provided example, the initial leaf value is '0x0000000000000000000000000000000000000000000000000000000000000000'.

Remember, comments can be added anywhere in the YAML file using the `#` symbol. For example, in the provided configuration, comments are used to label the groups as 'Orb' and 'Phone'. This can be particularly useful for providing additional context or explanations for your configuration options.

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

## 🚀 Advanced Usage

### Custom keys

The deployer will download the semaphore mtb binary and generate brand new keys automatically, as specified in the deployment configuration.

However it's possible to supplant custom keys (and even verifier contracts) - the deployer will not download generate keys or verifier contracts which are already present.

So to provide custom keys, make sure to place them in the cache directory (by default `.cache` under the deployment directory) and then:

1. Under `keys` for keys - keys filenames are expected to have the following format `keys_{mode}_{tree_depth}_{batch_size}`
2. Under `verifier_contracts` for contracts - contract filenames are expected to have the following format `{mode}_{tree_depth}_{batch_size}.sol`
