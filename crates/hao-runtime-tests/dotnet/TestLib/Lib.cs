using System;

namespace TestLib
{
    public class Calculator
    {
        private int total;

        public Calculator() : this(0) { }
        public Calculator(int start) { total = start; }

        public int Total => total;
        public int Add(int value) { total += value; return total; }
        public bool IsPositive() => total > 0;
        public string Describe(string prefix) => prefix + total;
        public Calculator Copy() => new Calculator(total);
        public void Fail(string message) => throw new InvalidOperationException(message);
        public string Nothing() => null;
        public char FirstChar(string value) => value[0];

        public static long Multiply(long a, long b) => a * b;
        public static double Half(double value) => value / 2;
        public static Calculator Combine(Calculator a, Calculator b) => new Calculator(a.total + b.total);
        public override string ToString() => $"Calculator({total})";
    }

    public static class Strings
    {
        public static string Reverse(string value)
        {
            var chars = value.ToCharArray();
            Array.Reverse(chars);
            return new string(chars);
        }

        public static int Length(string value) => value == null ? -1 : value.Length;
        public static string Describe(object value) => value == null ? "null" : value.GetType().Name;
    }

    namespace Greetings
    {
        public class Greeter
        {
            private readonly string greeting;
            public Greeter(string greeting) { this.greeting = greeting; }
            public string Greet(string name) => $"{greeting}, {name}!";
        }
    }
}
