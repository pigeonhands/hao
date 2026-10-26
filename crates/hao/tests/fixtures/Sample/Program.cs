using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Sample
{
    [AttributeUsage(AttributeTargets.All)]
    public sealed class TagAttribute : Attribute
    {
        public string Name { get; }
        public int Weight { get; set; }
        public TagAttribute(string name) { Name = name; }
    }

    public enum Colour : byte { Red = 1, Green = 2, Blue = 4 }

    [Flags]
    public enum Permissions { None = 0, Read = 1, Write = 2, Exec = 4 }

    public interface IShape
    {
        double Area();
        string Name { get; }
    }

    public readonly struct Point
    {
        public readonly int X;
        public readonly int Y;
        public Point(int x, int y) { X = x; Y = y; }
        public override string ToString() => $"({X}, {Y})";
    }

    [StructLayout(LayoutKind.Explicit, Size = 8)]
    public struct Union
    {
        [FieldOffset(0)] public long Whole;
        [FieldOffset(0)] public int Low;
        [FieldOffset(4)] public int High;
    }

    public abstract class Shape : IShape
    {
        public abstract double Area();
        public virtual string Name => GetType().Name;
    }

    [Tag("circle", Weight = 3)]
    public class Circle : Shape
    {
        public double Radius { get; set; }
        public Circle(double r) { Radius = r; }
        public override double Area() => Math.PI * Radius * Radius;
    }

    public class Box<T> where T : IComparable<T>
    {
        private readonly List<T> items = new List<T>();
        public event EventHandler Changed;
        public void Add(T item) { items.Add(item); Changed?.Invoke(this, EventArgs.Empty); }
        public T Max()
        {
            T best = items[0];
            foreach (var i in items) if (i.CompareTo(best) > 0) best = i;
            return best;
        }
        public TOut Map<TOut>(Func<T, TOut> f) => f(Max());

        public class Nested
        {
            public int Depth = 1;
        }
    }

    public static unsafe class Native
    {
        [DllImport("libc", EntryPoint = "getpid")]
        public static extern int GetPid();

        public static int Sum(int* values, int len)
        {
            int total = 0;
            for (int i = 0; i < len; i++) total += values[i];
            return total;
        }
    }

    public static class Calc
    {
        public static readonly int[] Primes = { 2, 3, 5, 7, 11, 13, 17, 19 };
        public const string Greeting = "Hello from hao";

        public static int Add(int a, int b) => a + b;

        public static int Classify(int v)
        {
            switch (v)
            {
                case 0: return 10;
                case 1: return 20;
                case 2: return 30;
                case 3: return 40;
                default: return -1;
            }
        }

        public static string Guarded(int v)
        {
            try
            {
                if (v == 0) throw new InvalidOperationException("zero");
                return "ok";
            }
            catch (InvalidOperationException e)
            {
                return "caught " + e.Message;
            }
            finally
            {
                Counter++;
            }
        }

        public static int Counter;

        public static ref int First(int[] arr) => ref arr[0];
        public static int[,] Grid() => new int[2, 3];
    }

    public static class Program
    {
        public static int Main(string[] args)
        {
            Console.WriteLine(Calc.Greeting);
            Console.WriteLine("add=" + Calc.Add(40, 2));
            Console.WriteLine("classify=" + Calc.Classify(2));
            Console.WriteLine("guarded=" + Calc.Guarded(0) + "," + Calc.Guarded(1) + "," + Calc.Counter);
            int primeSum = 0;
            foreach (var p in Calc.Primes) primeSum += p;
            Console.WriteLine("primes=" + primeSum);

            var box = new Box<int>();
            int changes = 0;
            box.Changed += (s, e) => changes++;
            box.Add(3); box.Add(9); box.Add(4);
            Console.WriteLine("max=" + box.Max() + " changes=" + changes + " map=" + box.Map(x => x * 2));

            IShape shape = new Circle(1.0);
            Console.WriteLine("shape=" + shape.Name + " " + shape.Area().ToString("F2"));
            var tag = typeof(Circle).GetCustomAttribute<TagAttribute>();
            Console.WriteLine("tag=" + tag.Name + ":" + tag.Weight);

            Console.WriteLine("colour=" + (Colour.Red | Colour.Blue) + " perms=" + (Permissions.Read | Permissions.Exec));
            Console.WriteLine("point=" + new Point(1, 2));
            var u = new Union { Whole = 0x0000000200000001 };
            Console.WriteLine("union=" + u.Low + "," + u.High);

            unsafe
            {
                int* vals = stackalloc int[] { 1, 2, 3 };
                Console.WriteLine("native=" + Native.Sum(vals, 3));
            }

            using (var s = Assembly.GetExecutingAssembly().GetManifestResourceStream("Sample.data.txt"))
            using (var r = new StreamReader(s))
            {
                Console.WriteLine("resource=" + r.ReadLine());
            }
            return 0;
        }
    }
}
