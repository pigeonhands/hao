hao
=======

(verb) (-a) to catch in a net, fish (with a net). 
> Kua taha ngā rā i hao ai i te ika o te moana, o te wai māori. / The days have passed to net the fish of the ocean and fresh water. 


<!-- [![Actions][actions-badge]][actions-url] -->

[![github-badge]][github-link]
[![crates-hao-badge]][crates-hao]
[![docs][docs-hao-badge]][docs-hao]
![License][license-badge]

[license-badge]:https://img.shields.io/crates/l/hao.svg?style=for-the-badge
[github-badge]: https://img.shields.io/badge/github-pigeonhands/hao-8da0cb?style=for-the-badge&labelColor=555555&logo=github
[github-link]: https://github.com/pigeonhands/hao
[actions-badge]: https://img.shields.io/github/actions/workflow/status/pigeonhands/hao/ci.yml?branch=master&style=for-the-badge
[actions-url]: https://github.com/pigeonhands/hao/actions
[crates-hao-badge]: https://img.shields.io/crates/v/hao.svg?style=for-the-badge&color=fc8d62&logo=rust
[crates-hao]: https://crates.io/crates/hao
[docs-hao-badge]: https://img.shields.io/badge/docs.rs-hao-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
[docs-hao]: https://docs.rs/hao


-----

Tools for working with .NET assemblies and runtimes from Rust.

| crate | |
|---|---|
| [`hao`](crates/hao) | read, edit, create and write .NET assemblies (like dnlib / Mono.Cecil) |
| [`hao-runtime`](crates/hao-runtime) | host CoreCLR or Mono in a Rust process and call managed code |
| [`hao-runtime-macros`](crates/hao-runtime-macros) | `bind!` generates Rust bindings to a .NET assembly |
| [`hao-il2cpp`](crates/hao-il2cpp) | read Unity Il2Cpp games (work in progress) |
| [`hao-runtime-tests`](crates/hao-runtime-tests) | binding tests (not published) |

hao's PE handling comes from [pewter](https://github.com/pigeonhands/pewter).

## Tests

```sh
cargo test --workspace
cargo test --workspace --release -- --include-ignored   # also rewrites the installed .NET framework
```

Tests that need the dotnet SDK, Mono, or game files skip themselves when those aren't available.
