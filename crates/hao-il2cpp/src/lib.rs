//! Reads Unity Il2Cpp games, to generate .NET proxy assemblies with hao.
//!
//! Work in progress: metadata v29 with ELF game binaries.
//!
//! [`Game`] is the entry point. It finds a game's `global-metadata.dat` and
//! `GameAssembly`, and gives types, fields and methods by name with their types
//! resolved:
//!
//! ```no_run
//! use hao_il2cpp::Game;
//!
//! let game = Game::open("path/to/game")?;
//! let string = game.find_type("System.String").unwrap();
//! for method in string.methods() {
//!     println!("{}", method); // `static string Concat(string str0, string str1)`
//! }
//! game.write_assemblies("out")?;
//! # Ok::<(), hao_il2cpp::Error>(())
//! ```
//!
//! Underneath, [`Metadata`] reads `global-metadata.dat`, [`Binary`] reads the
//! `Il2CppType` table from the game binary, and [`generate()`] builds the
//! declaration assemblies.

pub mod binary;
mod error;
pub mod game;
pub mod generate;
pub mod metadata;

pub use binary::{Binary, ElfImage, LoadedImage, PeImage};
pub use error::{Error, Result};
pub use game::{FieldView, Game, MethodView, ParameterView, TypeView};
pub use generate::{generate, GeneratedAssembly};
pub use metadata::Metadata;
