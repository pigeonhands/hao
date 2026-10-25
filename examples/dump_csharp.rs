use hao::{dotnet::csharp::CSharpOptions, Module};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: dump_csharp <assembly> [--il]");
    let options = CSharpOptions {
        il_bodies: args.any(|a| a == "--il"),
        ..Default::default()
    };

    let module = Module::from_path(path).unwrap();
    print!("{}", module.to_csharp_with(options));
}
