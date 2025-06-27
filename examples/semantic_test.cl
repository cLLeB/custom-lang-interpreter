// Test file for semantic analysis
print "Testing semantic analysis...";

// Valid code
let x = 10;
let y = 20;
let result = x + y;
print "Result: " + result;

// Type error - trying to add number and boolean
let bad_add = x + true;

// Undefined variable error
print undefined_var;

// Function with wrong argument count
print abs(1, 2, 3);

// Using non-function as function
let not_func = 42;
print not_func(5);

print "Semantic test completed";
