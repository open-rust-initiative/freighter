## Freighter - A simple crates registry

Freighter is a simple crates registry that can be used to host your own private crates. It's not a fork of [crates.io](https://crates.io), and it's not a replacement for it also.

### Why?

Usually, you don't need to host your own crate registry. When we are developing program using Rust in the company, we need to host our own crates registry for the following reasons:

1. We need to use some crates that are not published to crates.io, such as some internal crates.
2. We need to use some crates that are published to crates.io, but we need to modify them to fit our needs.
3. We need to use crates in our build system or CI/CD workflow.

### What's features?

1. Support sync crates index from crates.io.
2. Support cache crates file from crates.io.
3. Support publish crates to the registry.

### How to build?

### How to use?

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

* MIT LICENSE ( [LICESE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT) 
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

### Acknowledgements
