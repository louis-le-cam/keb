# Keb

IRs:
- TOK __tokens:__ list
- SYN __syntax:__ tree
- SEM __semantic:__ tree, typed
- INS __ssa:__ ssa, typed
- _MAC __codegen:__ 1:1 with asm, backend-specific_

# Zig

IRs:
- TOK __tokens:__ list
- SYN __ast:__ tree
- INS __zir:__ ssa-like
- INS __air:__ ssa, typed
- MAC __mir:__ 1:1 with asm, backend-specific

# Rust

IRS:
- TOK __tokens:__ list
- SYN __ast:__ tree
- SEM __hir:__ tree
- SEM __thir:__ tree, typed
- INS __mir:__ cfg, typed
