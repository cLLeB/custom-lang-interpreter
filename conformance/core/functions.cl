// Function definitions, recursion, parameters, and lambda expressions.
// vm: run
function fib(n) {
  if (n < 2) { return n }
  return fib(n - 1) + fib(n - 2)
}
print("fib(10): " + fib(10))

function add(a, b) {
  return a + b
}
print("add: " + add(20, 22))

let double = n => n * 2
print("lambda: " + double(21))
