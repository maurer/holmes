# Holmes

[![Build Status](https://travis-ci.org/BinaryAnalysisPlatform/holmes.svg?branch=master)](https://travis-ci.org/BinaryAnalysisPlatform/holmes)
[![Coverage Status](https://coveralls.io/repos/BinaryAnalysisPlatform/holmes/badge.svg)](https://coveralls.io/r/BinaryAnalysisPlatform/holmes)


A system for integrating multiple analyses using a logic language.

## Requirements
* **Rust** - Holmes is developed against [Rust Beta](https://static.rust-lang.org/dist/rust-1.0.0-beta-x86_64-unknown-linux-gnu.tar.gz)
  This may change when Rust releases 1.0.0 stable.

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
