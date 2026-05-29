// ── Manual test: custom-lang interpreter ─────────────────────────────────

// 1. Variables & basic types
let name = "World"
let age  = 25
let pi   = 3.14159
let flag = true
let nothing = null

print("Hello, " + name + "!")
print("Age: " + age)
print("Pi: " + pi)
print("Flag: " + flag)
print("Null: " + nothing)

// 2. Arithmetic
let x = 10
let y = 3
print(x + y)
print(x - y)
print(x * y)
print(x / y)
print(x % y)

// 3. If / else
if (age >= 18) {
    print("adult")
} else {
    print("minor")
}

// 4. While loop
let i = 0
while (i < 3) {
    print("loop " + i)
    i = i + 1
}

// 5. C-style for loop
for (let j = 0; j < 3; j += 1) {
    print("for " + j)
}

// 6. For-in over array
let arr = [10, 20, 30]
for (item in arr) {
    print("item: " + item)
}

// 7. Functions
function add(a, b) {
    return a + b
}
print(add(4, 5))

function greet(n) {
    return "Hello, " + n + "!"
}
print(greet("Caleb"))

// 8. Recursion (fibonacci)
function fib(n) {
    if (n <= 1) { return n }
    return fib(n - 1) + fib(n - 2)
}
print(fib(10))

// 9. Objects / maps
let person = { name: "Alice", age: 30 }
print(person.name)
print(person.age)

// 10. Nested function + closure
function make_counter() {
    let count = 0
    function inc() {
        count = count + 1
        return count
    }
    return inc
}
let counter = make_counter()
print(counter())
print(counter())
print(counter())

// 11. Boolean logic
print(true && false)
print(true || false)
print(!true)

// 12. Comparison
print(1 == 1)
print(1 != 2)
print(3 < 5)
print(5 > 3)

print("All tests done!")
