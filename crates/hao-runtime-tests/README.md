# hao-runtime-tests

Tests for [`hao-runtime-macros`](../hao-runtime-macros) bindings. Not published.

`build.rs` builds [`dotnet/TestLib`](dotnet/TestLib), a `netstandard2.0` library, with the dotnet SDK.
The same bindings are then run on CoreCLR (`tests/coreclr.rs`) and Mono (`tests/mono.rs`), each in
its own process since a process can only host one runtime of each kind. Without the SDK the tests
compile to nothing, and without Mono the Mono test skips itself.
