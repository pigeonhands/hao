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
[docs-hao]: https://docs.rs/crates/hao


-----

A libarary for reading and (eventually) writing .net assembiles and modules in rust.

## Example
```rust
use hao::{Module, Result};

fn main() -> Result<()>{
    let module = Metadata::from_path(r#"Example.Net.dll"#).unwrap();

    for ty in module.types().values() {
        println!("{} {{", ty.name());
        for field in ty.fields().values() {
            println!("\tfield: {}", field.name());
        }
        println!("}}");
    }

    Ok(())
}
```
You can see more examples in the [example](https://github.com/pigeonhands/hao/tree/master/examples) directory of the repository.