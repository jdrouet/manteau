<h1 align="center">Welcome to Manteau 👋</h1>
<p>
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000" />
</p>

> A fast and lightweight alternative to [Jackett](https://github.com/Jackett/Jackett/), written in Rust 🦀

The goal of manteau is to get rid of the weight of [Jackett](https://github.com/Jackett/Jackett/) that takes too much of ram, comes with a UI that might not be that important.

With Manteau, you **just** start the container and it works! You can also configure the indexers you want manteau to use.

## Install

```sh
# Using docker, compatible with amd64 and arm64 for now
docker run --name manteau -d -p 3000:3000 jdrouet/manteau:latest
```

You can then s

## Run tests

```sh
cargo --workspace test
```

## Configuration

You can specify the path to your configuration file using the `CONFIG_FILE` environment variable. By default it points to `./config.toml`. You can find the default configuration file at the root of this repository.

## Author

👤 **Jérémie Drouet**

- Website: https://www.buymeacoffee.com/jdrouet
- Github: [@jdrouet](https://github.com/jdrouet)

## Show your support

Give a ⭐️ if this project helped you!

---

_This README was generated with ❤️ by [readme-md-generator](https://github.com/kefranabg/readme-md-generator)_
