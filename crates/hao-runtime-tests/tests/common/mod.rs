mod test_lib {
    hao_runtime_macros::bind! {
        assembly_env = "HAO_TEST_LIB",
    }
}

use hao_runtime::{Error, Runtime};
use test_lib::test_lib::{greetings::Greeter, Calculator, Strings};

pub fn exercise(runtime: &Runtime) {
    let calculator = Calculator::new_1(runtime, 10).unwrap();
    assert_eq!(calculator.add(5).unwrap(), 15);
    assert_eq!(calculator.get_total().unwrap(), 15);
    assert!(calculator.is_positive().unwrap());
    assert_eq!(calculator.describe("total=").unwrap().as_deref(), Some("total=15"));
    assert_eq!(calculator.nothing().unwrap(), None);
    assert_eq!(calculator.first_char("hao").unwrap(), 'h' as u16);
    assert_eq!(calculator.object().to_managed_string().unwrap(), "Calculator(15)");

    let copy = calculator.copy().unwrap().unwrap();
    copy.add(1).unwrap();
    assert_eq!(calculator.get_total().unwrap(), 15);
    let combined = Calculator::combine(runtime, &calculator, &copy).unwrap().unwrap();
    assert_eq!(combined.get_total().unwrap(), 31);
    assert_eq!(Calculator::new(runtime).unwrap().get_total().unwrap(), 0);

    assert_eq!(Calculator::multiply(runtime, 6, 7).unwrap(), 42);
    assert_eq!(Calculator::half(runtime, 5.0).unwrap(), 2.5);

    match calculator.fail("boom") {
        Err(Error::Managed(message)) => assert!(message.contains("InvalidOperationException") && message.contains("boom"), "{}", message),
        other => panic!("expected a managed exception, got {:?}", other),
    }

    assert_eq!(Strings::reverse(runtime, "hello, 世界").unwrap().as_deref(), Some("界世 ,olleh"));
    assert_eq!(Strings::length(runtime, "").unwrap(), 0);
    assert_eq!(Strings::describe(runtime, calculator.object()).unwrap().as_deref(), Some("Calculator"));

    let greeter = Greeter::new(runtime, "Kia ora").unwrap();
    assert_eq!(greeter.greet("hao").unwrap().as_deref(), Some("Kia ora, hao!"));
}
