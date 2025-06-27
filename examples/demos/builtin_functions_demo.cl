// Built-in Functions Demo for Custom Language
print "=== Custom Language Built-in Functions Demo ===";

// 1. Math Functions
print "";
print "1. Math Functions:";

let negative = -15;
let positive = 25;
let decimal = 3.7;

print "abs(" + negative + ") = " + abs(negative);
print "abs(" + positive + ") = " + abs(positive);
print "abs(" + decimal + ") = " + abs(decimal);

print "sqrt(16) = " + sqrt(16);
print "sqrt(25) = " + sqrt(25);
print "sqrt(2) = " + sqrt(2);

print "pow(2, 3) = " + pow(2, 3);
print "pow(5, 2) = " + pow(5, 2);
print "pow(10, 0) = " + pow(10, 0);

print "min(10, 5) = " + min(10, 5);
print "min(-3, 7) = " + min(-3, 7);
print "max(10, 5) = " + max(10, 5);
print "max(-3, 7) = " + max(-3, 7);

// 2. String Functions
print "";
print "2. String Functions:";

let greeting = "Hello";
let name = "World";
let empty = "";

print "len(" + greeting + ") = " + len(greeting);
print "len(" + name + ") = " + len(name);
print "len(" + empty + ") = " + len(empty);

// 3. Type Inspection
print "";
print "3. Type Inspection:";

let num = 42;
let str = "text";
let bool = true;
let nothing = null;

print "type(" + num + ") = " + type(num);
print "type(" + str + ") = " + type(str);
print "type(" + bool + ") = " + type(bool);
print "type(" + nothing + ") = " + type(nothing);

// 4. Practical Examples
print "";
print "4. Practical Examples:";

// Calculate hypotenuse using built-in functions
function hypotenuse(a, b) {
    return sqrt(pow(a, 2) + pow(b, 2));
}

// Check if a number is within a range
function inRange(value, minVal, maxVal) {
    return value >= min(minVal, maxVal) && value <= max(minVal, maxVal);
}

// Calculate circle properties
function circleInfo(radius) {
    let pi = 3.14159;
    let area = pi * pow(radius, 2);
    let circumference = 2 * pi * radius;
    
    print "Circle with radius " + radius + ":";
    print "  Area: " + area;
    print "  Circumference: " + circumference;
}

print "Hypotenuse of 3-4-5 triangle: " + hypotenuse(3, 4);
print "Is 15 in range [10, 20]? " + inRange(15, 10, 20);
print "Is 5 in range [10, 20]? " + inRange(5, 10, 20);

circleInfo(5);

// 5. Error Handling Examples
print "";
print "5. Error Handling Examples:";

// These would cause errors if uncommented:
// print sqrt(-1);  // Error: negative square root
// print pow("text", 2);  // Error: non-numeric arguments
// print len(42);  // Error: len() requires string

print "Built-in functions demo completed successfully!";
