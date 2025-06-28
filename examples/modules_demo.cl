// Test module system
print "=== Module System Demo ===";
print "";

// Import the math utilities module
import "examples/math_utils";

print "Testing imported functions:";

// Test the imported functions
let result1 = add(5, 3);
print "add(5, 3) = " + result1;

let result2 = multiply(4, 6);
print "multiply(4, 6) = " + result2;

let result3 = square(7);
print "square(7) = " + result3;

print "Testing imported constants:";
print "PI = " + PI;
print "E = " + E;

print "";
print "Module system demo complete!";
