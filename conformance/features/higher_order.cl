// Functions are first-class: passed as arguments and returned from functions.
function apply_twice(f, x) {
  return f(f(x))
}
let inc = n => n + 1
print("apply_twice: " + apply_twice(inc, 10))

function adder(n) {
  return x => x + n
}
let add5 = adder(5)
print("add5: " + add5(100))
