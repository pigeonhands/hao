//! Loads an assembly and writes it back out with rebuilt metadata.

use hao::Module;

fn main() {
    let mut args = std::env::args().skip(1);
    let (input, output) = match (args.next(), args.next()) {
        (Some(i), Some(o)) => (i, o),
        _ => panic!("usage: rewrite <input> <output>"),
    };
    let module = Module::from_path_no_resolve(&input).unwrap();
    module.write_to_path(&output).unwrap();
    println!("wrote {}", output);
}
