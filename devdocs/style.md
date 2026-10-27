## Brooks Style

These are the patterns that we _try_ to follow in the Brooks library. We believe
that they generally adhere to the best practices. If you find something that we
could be doing better, please let us know!

### Error Types

In general, all error types should go into their own modules. The exception are
error types that are specific to a trait. Those error types should be defined in
the same module as the trait.

### Prefixes on Names

There should be no prefixes on names (e.g., use `CompilerError` rather than
`MelCompilerError`). A user of a "common name" outside the module where it is
declared (e.g., `InterpError` in the `mel` module) should disambiguate using
`as` on the import _or_ by using some/all of the module path.
