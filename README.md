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
There are several commands you can run to sync index and rustup toolchains with freighter, 
__freighter crates pull__ will start clone a git index from upstream to local, which preserves all the crates information.
__freighter crates download__ will download the crates files parsed from git index you pulled. If you're first running freighter, you need to add a flag __--init__ after download command for init download all the files, otherwise it will only download incremental file by last __pull command__.It may take some times if you use init flag , but you can use __-c augumment__ to use more threads or change the default value in __config.toml__ to speed up download. 

Crates File initlization:

```bash
docker exec freighter bash -c 'freighter crates pull && freighter crates download --init'
```

It is better to upldate the index and download file frequently for incremental updates by combine using these commands:

```bash
docker exec freighter bash -c 'freighter crates pull && freighter crates download'
```

After download crates file, you can also sync rustup mirrors if that meets your requirements:

```bash
docker exec freighter bash -c 'freighter rustup download'
and 
docker exec freighter bash -c 'freighter channel download'
```

After download all the files by using __freighter crates download__ and __freighter rustup download__, you can run upload command to upstream all your local files to s3, for example:

```bash
// create your own bucket in s3
s3cmd mb s3://your-own-bucket

//start uplaod to your own bucket
freighter crates upload --bucket your-own-bucket
freighter rustup upload --bucket your-own-bucket
```
Tips: we use s3cmd to upload files, so you may need to complete your own configuration before using [s3cmd](https://github.com/s3tools/s3cmd)

##### 4. Add the cron job to the crontab.
```bash
$ crontab -e
$ # Add the following line to the crontab file
$ */5 * * * * docker exec freighter bash -c 'freighter crates pull && freighter crates download'
$ 0 2 * * *  docker exec freighter bash -c 'freighter rustup download'
```

#### Directly Usage

##### 1. Download the crates index with specify directory
```bash
$ freighter -c /mnt/volume_fra1_01 crates pull
```
##### 2. Download all crates file to local disk and then uoload to s3(you need to config s3cmd tools):
```bash
freighter crates download --init --upload
```
##### 3. Download crates file with multi-thread to specify directory:
```bash
freighter -c /mnt/volume_fra1_01 crates -t 128 download --init
```

##### 4. Download rustup init file:
```bash
freighter -c /mnt/volume_fra1_01 rustup -t 128 download
```

##### 5. Download rust toolchain files:
```bash
freighter -c /mnt/volume_fra1_01 channel -t 128 download
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
