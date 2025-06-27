// Test built-in functions and user-defined functions
print "Testing built-in functions:";

// Math functions
let x = -5;
print abs(x);

let y = 16;
print sqrt(y);

print pow(2, 3);
print min(10, 5);
print max(10, 5);

// String functions
let message = "Hello";
print len(message);

// Type function
print type(42);
print type("hello");
print type(true);

print "Testing user-defined functions:";

// Simple function
function add(a, b) {
    return a + b;
}

let result = add(10, 20);
print result;

// Recursive function
function factorial(n) {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

print factorial(5);

// Fibonacci function
function fib(n) {
    if (n <= 1) {
        return n;
    } else {
        return fib(n - 1) + fib(n - 2);
    }
}

print fib(8);
