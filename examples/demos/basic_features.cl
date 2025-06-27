// Basic Features Demo for Custom Language
print "=== Custom Language Basic Features Demo ===";

// 1. Variables and Data Types
print "";
print "1. Variables and Data Types:";
let number = 42;
let text = "Hello, World!";
let flag = true;
let nothing = null;

print "Number: " + number;
print "Text: " + text;
print "Boolean: " + flag;
print "Null: " + nothing;

// 2. Arithmetic Operations
print "";
print "2. Arithmetic Operations:";
let a = 10;
let b = 3;

print "a = " + a + ", b = " + b;
print "a + b = " + (a + b);
print "a - b = " + (a - b);
print "a * b = " + (a * b);
print "a / b = " + (a / b);
print "a % b = " + (a % b);

// 3. Comparison Operations
print "";
print "3. Comparison Operations:";
print "a == b: " + (a == b);
print "a != b: " + (a != b);
print "a > b: " + (a > b);
print "a < b: " + (a < b);
print "a >= b: " + (a >= b);
print "a <= b: " + (a <= b);

// 4. Logical Operations
print "";
print "4. Logical Operations:";
let x = true;
let y = false;
print "x = " + x + ", y = " + y;
print "x && y: " + (x && y);
print "x || y: " + (x || y);
print "!x: " + (!x);
print "!y: " + (!y);

// 5. Control Flow - If/Else
print "";
print "5. Control Flow - If/Else:";
if (a > b) {
    print "a is greater than b";
} else {
    print "a is not greater than b";
}

let score = 85;
if (score >= 90) {
    print "Grade: A";
} else if (score >= 80) {
    print "Grade: B";
} else if (score >= 70) {
    print "Grade: C";
} else {
    print "Grade: F";
}

// 6. Control Flow - While Loops
print "";
print "6. Control Flow - While Loops:";
let counter = 0;
print "Counting to 5:";
while (counter < 5) {
    print "Count: " + counter;
    counter = counter + 1;
}

print "";
print "Demo completed successfully!";
