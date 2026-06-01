// expect-error: type mismatch
// A typed parameter seeds the checker: using `n: number` to initialize a
// `string`-annotated local is rejected inside the function body.
function bad(n: number) {
  let s: string = n
  return s
}
print(bad(5))
