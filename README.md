# shhhfs

Virtual file system for secrets.

It allows using secret managers like 1Password, as it was a directory with files, which allows approving access to files with biometric authentication.

🚧 Work in progress.

## Motivation

Switching from keeping secrets in Git-ignored `.env` to encrypted secrets setup, e.g., [age](https://age-encryption.org/) + [fnox](https://fnox.jdx.dev/), adds an extra layer of security but is ultimately flawed as it still requires the secret age key to be exposed as `FNOX_AGE_KEY` or `FNOX_AGE_KEY_FILE` env var.

shhhfs solves this problem by mounting secrets as a virtual file system and allowing access only with the user's approval, e.g., biometric authentication.

## Installation

### Linux

To install shhhfs on Linux, you need to have [FUSE](https://www.kernel.org/doc/html/next/filesystems/fuse.html) installed, e.g.,:

```bash
sudo apt install fuse
```

Then install shhhfs using Cargo:

```bash
cargo install shhhfs
```

### macOS

shhhfs relies on [macFUSE](https://osxfuse.github.io/) to create a virtual file system, so you need to install it first:

```bash
brew install pkgconf macfuse
```

> See [macFUSE's Getting Started](https://github.com/macfuse/macfuse/wiki/Getting-Started) for more details on how to install and set up macFUSE.

Then install shhhfs using Cargo:

```bash
cargo install shhhfs
```

## Usage

### Example

To see shhhfs in action, you can use the `json` provider, which simply uses a JSON object as the source for the virtual file system. **Use it only for testing, as it doesn't provide any security.**

Run `shhhfs` with the `mount` command:

```bash
shhhfs mount --provider json --provider-opts '{"hello.txt": "Hello, cruel world!"}' ~/.shhfs
```

Then you can access the `hello.txt` file in the `~/.shhfs` directory:

```bash
ls ~/.shhfs
#=> hello.txt

cat ~/.shhfs/hello.txt
#=> Hello, cruel world!
```
