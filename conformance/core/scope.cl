// Block scoping and shadowing: the inner binding does not leak out.
// vm: run
let x = 1
{
  let x = 2
  print("inner: " + x)
}
print("outer: " + x)
