use hao::{
    dotnet::csharp::{CSharpOptions, WriteCSharp},
    Module,
};

const SAMPLE: &[u8] = include_bytes!("fixtures/Sample.dll");

#[test]
fn declarations() {
    let module = Module::from_bytes(SAMPLE).unwrap();
    let source = module.to_csharp();
    for expected in [
        "public enum Colour : byte",
        "    Blue = 4,",
        "public readonly struct Point",
        "public abstract class Shape : IShape",
        "public class Box<T> where T : IComparable<T>",
        "public event EventHandler Changed;",
        "public TOut Map<TOut>(Func<T, TOut> f);",
        "public class Nested\n",
        "public static extern int GetPid();",
        "public const string Greeting = \"Hello from hao\";",
        "public static ref int First(int[] arr);",
        "public static int[,] Grid();",
        "public override double Area();",
    ] {
        assert!(source.contains(expected), "missing {:?} in\n{}", expected, source);
    }
}

#[test]
fn display_adapter_and_il() {
    let module = Module::from_bytes(SAMPLE).unwrap();
    let calc = module.find_type("Sample", "Calc").unwrap();
    let add = calc.get().find_method("Add").unwrap();
    assert_eq!(add.csharp().to_string(), "public static int Add(int a, int b);\n");

    let source = module.to_csharp_with(CSharpOptions {
        il_bodies: true,
        ..Default::default()
    });
    assert!(source.contains("// IL_0002: add"));
    assert!(source.contains("// IL_0001: ldstr \"Hello from hao\"") || source.contains("ldstr \"Hello from hao\""));
    assert!(source.contains("finally handler"));
}

#[test]
#[ignore]
fn write_corlib() {
    let dir = hao::resolver::PathAssemblyResolver::find_framework_dir().unwrap();
    let module = Module::from_path_no_resolve(dir.join("System.Private.CoreLib.dll")).unwrap();
    let source = module.to_csharp_with(CSharpOptions {
        il_bodies: true,
        ..Default::default()
    });
    assert!(source.contains("public abstract class Enum"));
}
