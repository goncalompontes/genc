1. Purpose & scope

`genc` is meant to be a reimagined `C` with some new additional features.
It is mainly a learning/hobby project for me to get into real world compiler dev for imperative languages.

2. Compilation model

The language will compile down to native, through a multi-pass architecture. The idea is to eventually build something "similar" in spirit to LLVM but **very** much cut down on scope.
It is ahead-of-time compiled and is planned to _eventually_ support some form of incremental compilation.

3. Syntax & grammar

The syntax of the language is not yet decided as i plan to first have the `core` language setup and then add the `syntax` layer on top. This prevents me from bikeshedding as usual with the irrelevant frontend related things.

4. Type System

The language will be statically and strongly typed, with a much more advanced inference algorithm then `C` and `C++`.
How exactly this inference will work is not yet decided but will be documented as progress is made.
Typing will be nominal, the primitives will closely match the ones provided by C but with clearer names and purposes.
The language will support monomorphized generics alongside type classes
We will support product types (struct), sum types (union), and tagged unions (variant).
We will also support pattern matching on tagged unions
There will be no form of subtyping, implicit conversions or anything of the sort. casts however will be possible but how this will be implemented is not yet specified.
