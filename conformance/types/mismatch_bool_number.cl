// expect-error: type mismatch
// A number initializer for a `bool`-annotated binding is rejected statically.
let flag: bool = 3
print(flag)
