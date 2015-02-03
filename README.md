# Holmes

![Holmes Build Badge](https://travis-ci.org/maurer/holmes.svg?branch=rust)

A system for integrating multiple analyses using a logic language.

## Requirements
* **Rust** - Holmes is developed against [Rust Nightly](https://static.rust-lang.org/dist/rust-nightly-x86_64-unknown-linux-gnu.tar.gz).
  This may change when Rust releases 1.0.0 stable.
  For now, if Holmes doesn't build, and the build-badge says it does, try updating your Rust first as it moves quickly.

* **PostgreSQL** - Holmes uses **PostgreSQL** to back its datastore.
  I develop against 9.4, and test against 9.3 on [Travis](https://travis-ci.org/maurer/holmes).
  However, there should not be a strong version dependency, and other versions will likely work.
  Other backing stores may become available in the future.

* **Cap'n Proto** - Holmes uses [Cap'n Proto](https://capnproto.org/) to provide RPC and capability support.
  I develop and test against the latest git master of [Cap'n Proto](https://github.com/sandstorm-io/capnproto).
  It should still work with the latest [release](https://capnproto.org/capnproto-c++-0.5.1.tar.gz), but this is not the dev/test environment.

* **Linux/X86_64** - This is not an explicit dependency.
  To the best of my knowledge, all tools I am using work on OSX/Windows and I am not using any architecture specific hacks.
  However, I am not developing on other architectures/OSes, nor will I be testing on them until things are much more feature complete.
