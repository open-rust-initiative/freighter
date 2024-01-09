## Freighter - Rust Proxy and Registry

The Freighter's purpose is to help the community and company to build the proxy for crates.io and the registry for the development environment.

### Why need the Freighter?

When developing a program using Rust in a company, we need to host a proxy for crates.io and private crates registry for the following reasons:

- The developers will only be allowed use crates of the company with security and complete evaluation.
- Some crates need to upgrade functions or fix bugs, and the new version does not allow developers to contribute upstream.
- Some private crates share with different teams and products in the development process.

### What are the features?

- The Freighter is a crate registry for private crates, public crates index and crates sync from crates.io. The registry can store the files in the local disk or storage service compliance S3.
- The Freighter has an analytics engine for rating public crates.
- The Freighter has a blacklist and whitelist of evaluated crates used in DevOps.

### How to use the Freighter?

Freighter's functionality mainly consists of four parts: synchronizing crates index and crates; syncing the rustup-init files; syncing the rust toolchain files; providing a HTTP server that support static file server, parse the Git protocol, and offering API services such as crate publication.

Freighter can be executed as a standalone executable program. You can build it using the **cargo build --release** command and then copy it to your `/usr/local/bin directory`.

#### 1. Prerequisite
* Freighter defaults to storing data in the **default directory /Users/${USERNAME}/freighter**. To customize the storage directory, modify the config.toml file.
* The config file can be obtained by executing any command; Freighter copies the config.default.toml file to the default directory.
* Alternatively, the config file can be manually copied from the source code directory src which named **config.default.toml**.
* Customize the storage path for data by modifying configurations like **log_path, index_path, crates_path**, etc., in the config file.
* Freighter uses the config.toml configuration in the default directory for its operations. To use a custom config path, add the -c parameter when executing a command. For example:
  ```bash
  freighter -c /path/to/config.toml <subcommand>
  ```
    You can specify paths like /tmp/freighter/config.toml, /tmp/freighter, or /tmp, and Freighter will automatically interpret them as /tmp/freighter/config.toml.

#### 2. Synchronizing Crates Index and Crates
To sync crate files, Freighter needs to first sync the crates index. You can use the following command to sync the index file:

```bash
freighter crates pull
```

This command will create a crates.io-index directory in the default path **/Users/${USERNAME}/freighter** and fetch the index. If the index already exists, it will attempt to update it. 

**Full download**: Next, you can use the download command with the init parameter to download the full set of crates files:

```bash
freighter crates download --init
```

**Incremental update**: Without the init parameter, Freighter will compare log records in the **working directory** to determine the index and crates that need incremental updates:

```bash
freighter crates download
```

#### 3.Syncing the rustup-init Files
#### 4.Syncing the Rust Toolchain Files
#### 5.Http Server

### How to contribute?

This project enforce the [DCO](https://developercertificate.org).

Contributors sign-off that they adhere to these requirements by adding a Signed-off-by line to commit messages.

```bash
This is my commit message

Signed-off-by: Random J Developer <random@developer.example.org>
```

Git even has a -s command line option to append this automatically to your commit message:

```bash
$ git commit -s -m 'This is my commit message'
```

### License

Freighter is licensed under this Licensed:

* MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT) 
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

### Acknowledgements
