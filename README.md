# AurSearchBot

[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)
[![LICENSE](https://img.shields.io/github/license/alenpaul2001/AurSearchBot?color=%2340AA8B)](./LICENSE.md)
[![Try it on telegram](https://img.shields.io/badge/try%20it-on%20telegram-0088cc.svg)](http://t.me/AurSearchBot)

A Telegram Inline Search Bot Written in Rust

# Introduction

Telegram Bot that can search [AUR](https://aur.archlinux.org/) ( Arch User Repository ) in inline mode. This bot make use of AUR's [RPC interface](https://aur.archlinux.org/rpc.php) to find packages.


### Building & Running

build using `cargo build`
```sh
git clone https://gitlab.com/alenpaul2001/aursearchbot.git
cd aursearchbot
cargo build
```

set environment variable `TELOXIDE_TOKEN` to BOT_TOKEN 
which you can get from @Botfather then run the target binary.
alternatively you can use cargo run<br>
`TELOXIDE_TOKEN="12345:49dc3eeb1aehda3cI2TesHNHc" cargo run`

### Copyright & License

* Copyright (C) 2023 by [AlenPaulVarghese](https://github.com/alenpaulvarghese)
* Licensed under the terms of the [BSD 3-Clause License](./LICENSE.md)
