## Freighter - Crates Infrastructure for Local Development

[Freighter](https://github.com/open-rust-initiative/freighter) is an open source project to helping build the DevOps infrastructure for proxying the [crates.io](https://crates.io) and provide simple registry functionality for local development.

### Why?

Usually, you don't need to host your own crate registry. When we are developing program using Rust in the company, we need to host private crates registry for the following reasons:

1. We need to use some crates that are not published to crates.io, such as some internal crates.
2. We need to use some crates that are published to crates.io, but we need to modify them to fit our needs.
3. We need to use crates in our build system or CI/CD workflow.

### What's the major features?

1. Support sync crates index from crates.io.
2. Support cache crates file from crates.io.
3. Support publish crates into the registry.

### How to use?

#### Deploy Freighter With Docker

##### 1. Pull docker image from your registry.
```bash
docker pull registry.digitalocean.com/rust-lang/freighter:latest
```

##### 2. Start dokcer container
before start wo should grant permission to your __workdir__, in the following example which is /mnt/volume_fra1_02.

```bash
chmod 777 /mnt/volume_fra1_02
```
Then start container with your own volume.
```bash
docker run -it -d -v /mnt/volume_fra1_02/:/freighter  --name freighter registry.digitalocean.com/rust-lang/freighter:latest
```

##### 3. Start downlaod files and upload to s3.
There are several commands you can run to sync data with freighter, for example if you want to sync crates,you should first run __freighter sync pull__ and then __freighter sync download__

```bash
docker exec freighter bash -c 'freighter sync pull && freighter sync download'
```

if you want to sync rustup mirrors, just run 
```bash
docker exec freighter bash -c 'freighter sync rustup'
```

After download all the files by using __freighter sync download__ and __freighter sync rustup__, you can run upload command to upstream all your local files to s3, for example:

```bash
// create your own bucket in s3
s3cmd mb s3://your-own-bucket

//start uplaod to your own bucket
freighter sync upload --bucket your-own-bucket
```
Tips: we use s3cmd to upload files, so you may need to complete your own configuration before using [s3cmd](https://github.com/s3tools/s3cmd)

##### 4. Add the cron job to the crontab.
```bash
$ crontab -e
$ # Add the following line to the crontab file
$ */5 * * * * docker exec freighter bash -c 'freighter sync pull && freighter sync download'
$ 0 2 * * *  docker exec freighter bash -c 'freighter sync rustup'
```

#### Directly Usage

##### 1. Sync the crates index with specify directory
```bash
$ freighter sync -c /mnt/volume_fra1_01 pull
```
##### 2. Download all crates file to local disk and then uoload to s3(you need to config s3cmd tools):
```bash
freighter sync download --init --upload
```
##### 3. Download crates file with multi-thread to specify directory:
```bash
freighter sync -t 128 -c /mnt/volume_fra1_01 download --init
```

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
