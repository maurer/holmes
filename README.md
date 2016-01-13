# Holmes

[![Build Status](https://travis-ci.org/maurer/holmes.svg?branch=master)](https://travis-ci.org/maurer/holmes)
[![Coverage Status](https://coveralls.io/repos/maurer/holmes/badge.svg)](https://coveralls.io/r/maurer/holmes)


A system for integrating multiple analyses using a logic language.

## Requirements
* **Rust** - Holmes is developed against [Rust 1.5.0](https://static.rust-lang.org/dist/rust-1.5.0-x86_64-unknown-linux-gnu.tar.gz).

* **PostgreSQL** - Holmes uses **PostgreSQL** to back its datastore.
  I develop against 9.4, and test against 9.3 on [Travis](https://travis-ci.org/maurer/holmes).
  However, there should not be a strong version dependency, and other versions will likely work.
  Other backing stores may become available in the future.

* **Linux/X86_64** - This is not an explicit dependency.
  To the best of my knowledge, all tools I am using work on OSX/Windows and I am not using any architecture specific hacks.
  However, I am not developing on other architectures/OSes, nor will I be testing on them until things are much more feature complete.
