// Ok, before starting any sort of typechecking, we need to do name resolution
// The first step of name resolution is doing a pass over the AST and extracting all definitions
// This means collecting names of functions, types and global variables. we don't *currently* have globals but will add them later
// What we do is we walk the entire AST, find definitions, and add them to the TypeEnvironment
// the environment is a map from a name to a type. for user defined types, that type has yet another layer of indirection in the form of
// a type id. a type id is a key to another data structure that we have called the TypeRegistry, which is where the definitions of type exist

pub mod error;
pub mod kvec;
pub mod nameres;
pub mod parser;
