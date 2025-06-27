// Performance Test Program
// Tests various language features for benchmarking
print "=== Performance Test Suite ===";

// Test 1: Basic arithmetic operations
print "Test 1: Arithmetic Operations";

let a = 100;
let b = 200;
let c = 300;

let result1 = a + b + c;
let result2 = a * b * c;
let result3 = a - b + c;
let result4 = a / b * c;

print "Arithmetic results: " + result1 + ", " + result2 + ", " + result3 + ", " + result4;

// Test 2: Function calls
print "Test 2: Function Calls";

function simple_add(x, y) {
    return x + y;
}

function simple_multiply(x, y) {
    return x * y;
}

function nested_calculation(a, b, c) {
    let temp1 = simple_add(a, b);
    let temp2 = simple_multiply(temp1, c);
    return temp2;
}

let func_result1 = simple_add(10, 20);
let func_result2 = simple_multiply(5, 6);
let func_result3 = nested_calculation(2, 3, 4);

print "Function results: " + func_result1 + ", " + func_result2 + ", " + func_result3;

// Test 3: Variable assignments and lookups
print "Test 3: Variable Operations";

let var1 = 1;
let var2 = 2;
let var3 = 3;
let var4 = 4;
let var5 = 5;

let sum = var1 + var2 + var3 + var4 + var5;
print "Variable sum: " + sum;

// Test 4: Control flow
print "Test 4: Control Flow";

let counter = 0;
let limit = 5;

if (counter < limit) {
    counter = counter + 1;
    if (counter < limit) {
        counter = counter + 1;
        if (counter < limit) {
            counter = counter + 1;
        }
    }
}

print "Counter final value: " + counter;

// Test 5: String operations
print "Test 5: String Operations";

let str1 = "Hello";
let str2 = "World";
let str3 = "!";

let combined = str1 + " " + str2 + str3;
print "String result: " + combined;
print "String length: " + len(combined);

// Test 6: Built-in function calls
print "Test 6: Built-in Functions";

let math_result1 = abs(-50);
let math_result2 = sqrt(100);
let math_result3 = pow(2, 10);
let math_result4 = min(100, 200);
let math_result5 = max(100, 200);

print "Math results: " + math_result1 + ", " + math_result2 + ", " + math_result3;
print "Min/Max results: " + math_result4 + ", " + math_result5;

// Test 7: Simple calculation (no loops)
print "Test 7: Manual Calculation";

function sum_to_5() {
    return 1 + 2 + 3 + 4 + 5;
}

let manual_result = sum_to_5();
print "Manual sum 1+2+3+4+5 = " + manual_result;

// Test 8: Complex expressions
print "Test 8: Complex Expressions";

let complex1 = (a + b) * (c - a) / (b + 1);
let complex2 = abs(a - b) + sqrt(c) - pow(2, 3);
let complex3 = min(max(a, b), c) + len("test");

print "Complex results: " + complex1 + ", " + complex2 + ", " + complex3;

// Test 9: Boolean operations
print "Test 9: Boolean Logic";

let bool1 = true;
let bool2 = false;
let bool3 = a > b;
let bool4 = c == 300;

let logic1 = bool1 && bool2;
let logic2 = bool1 || bool2;
let logic3 = !bool1;
let logic4 = bool3 && bool4;

print "Boolean results: " + logic1 + ", " + logic2 + ", " + logic3 + ", " + logic4;

// Test 10: Type checking
print "Test 10: Type Operations";

let type1 = type(42);
let type2 = type("string");
let type3 = type(true);
let type4 = type(null);

print "Types: " + type1 + ", " + type2 + ", " + type3 + ", " + type4;

print "=== Performance Test Complete ===";
print "This test exercises:";
print "- Arithmetic operations";
print "- Function definitions and calls";
print "- Variable assignments and lookups";
print "- Control flow (if/else, while)";
print "- String operations and concatenation";
print "- Built-in function calls";
print "- Manual calculations";
print "- Complex expressions";
print "- Boolean logic";
print "- Type checking";
print "";
print "Use this with 'time' command or --verbose flag for performance analysis.";
