// Simple Calculator Program
print "=== Simple Calculator ===";

// Basic arithmetic functions
function add(a, b) {
    return a + b;
}

function subtract(a, b) {
    return a - b;
}

function multiply(a, b) {
    return a * b;
}

function divide(a, b) {
    if (b == 0) {
        print "Error: Division by zero!";
        return null;
    }
    return a / b;
}

// Simple mathematical functions (no loops)
function factorial_5() {
    return 5 * 4 * 3 * 2 * 1;
}

function power_2_3() {
    return 2 * 2 * 2;
}

// Simple utility functions
function is_even(n) {
    return n % 2 == 0;
}

function is_odd(n) {
    return n % 2 == 1;
}

function square(n) {
    return n * n;
}

function cube(n) {
    return n * n * n;
}

// Demonstration calculations
print "";
print "Basic Arithmetic:";
print "15 + 7 = " + add(15, 7);
print "20 - 8 = " + subtract(20, 8);
print "6 * 9 = " + multiply(6, 9);
print "84 / 12 = " + divide(84, 12);

print "";
print "Mathematical Functions:";
print "5! = " + factorial_5();
print "2^3 = " + power_2_3();
print "Square of 7 = " + square(7);
print "Cube of 4 = " + cube(4);

print "";
print "Number Properties:";
print "Is 8 even? " + is_even(8);
print "Is 7 odd? " + is_odd(7);
print "Is 15 even? " + is_even(15);

print "";
print "Built-in Math Functions:";
print "abs(-42) = " + abs(-42);
print "sqrt(144) = " + sqrt(144);
print "pow(3, 4) = " + pow(3, 4);
print "min(15, 23) = " + min(15, 23);
print "max(15, 23) = " + max(15, 23);

print "";
print "Calculator demo completed!";
