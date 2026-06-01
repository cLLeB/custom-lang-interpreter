// expect-error: type mismatch
// A string initializer for a `number`-annotated binding is rejected statically.
let count: number = "not a number"
print(count)
