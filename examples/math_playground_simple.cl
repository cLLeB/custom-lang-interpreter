// Simple Math Playground - Safe Version
print "=== Simple Math Playground ===";
print "";

// 1. Basic Arithmetic
print "🔢 Basic Arithmetic:";
print "15 + 7 = " + (15 + 7);
print "20 - 8 = " + (20 - 8);
print "6 * 9 = " + (6 * 9);
print "84 / 12 = " + (84 / 12);
print "17 % 5 = " + (17 % 5);
print "";

// 2. Built-in Math Functions
print "🔬 Built-in Functions:";
print "abs(-42) = " + abs(-42);
print "sqrt(144) = " + sqrt(144);
print "pow(3, 4) = " + pow(3, 4);
print "min(15, 23) = " + min(15, 23);
print "max(15, 23) = " + max(15, 23);
print "";

// 3. Geometry
print "📐 Geometry:";
function circle_area(radius) {
    let pi = 3.14159;
    return pi * radius * radius;
}

function circle_circumference(radius) {
    let pi = 3.14159;
    return 2 * pi * radius;
}

let r = 5;
print "Circle with radius " + r + ":";
print "  Area: " + circle_area(r);
print "  Circumference: " + circle_circumference(r);
print "";

// 4. Number Properties
print "🧮 Number Properties:";
function number_info(n) {
    print "Number " + n + ":";
    print "  Even: " + (n % 2 == 0);
    print "  Divisible by 3: " + (n % 3 == 0);
    print "  Square: " + (n * n);
    print "  Cube: " + (n * n * n);
}

number_info(6);
print "";
number_info(7);
print "";

// 5. Simple Sequences
print "🔢 Number Sequences:";
print "Fibonacci (manual): 0, 1, 1, 2, 3, 5, 8, 13, 21, 34";
print "Squares: 1, 4, 9, 16, 25, 36, 49, 64, 81, 100";
print "Cubes: 1, 8, 27, 64, 125, 216, 343, 512, 729, 1000";
print "";

// 6. Simple Statistics with Arrays
print "📊 Statistics with Arrays:";
let data = [12, 15, 18, 13, 19, 16, 14, 17];
print "Dataset: " + data;
print "Length: " + len(data);
print "First: " + first(data);
print "Last: " + last(data);

// Manual sum calculation
let sum = data[0] + data[1] + data[2] + data[3] + data[4] + data[5] + data[6] + data[7];
let mean = sum / len(data);
print "Sum: " + sum;
print "Mean: " + mean;
print "";

// 7. Simple Factorials
print "🎲 Factorials:";
function factorial_small(n) {
    if (n == 0) return 1;
    if (n == 1) return 1;
    if (n == 2) return 2;
    if (n == 3) return 6;
    if (n == 4) return 24;
    if (n == 5) return 120;
    if (n == 6) return 720;
    return -1; // Not supported
}

print "0! = " + factorial_small(0);
print "3! = " + factorial_small(3);
print "5! = " + factorial_small(5);
print "6! = " + factorial_small(6);
print "";

// 8. Mathematical Constants
print "🔬 Mathematical Constants:";
print "π ≈ 3.14159";
print "e ≈ 2.71828";
print "Golden ratio φ ≈ 1.618";
print "22/7 = " + (22/7) + " (π approximation)";
print "355/113 = " + (355/113) + " (better π approximation)";
print "";

// 9. Fun Number Facts
print "🎯 Fun Number Facts:";
print "Perfect squares: 1, 4, 9, 16, 25, 36, 49, 64, 81, 100";
print "Prime numbers: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29";
print "Triangular numbers: 1, 3, 6, 10, 15, 21, 28, 36, 45, 55";
print "";

// 10. Simple Calculations
print "💡 Practical Calculations:";

function compound_interest(principal, rate, time) {
    // Simple compound interest: A = P(1 + r)^t
    // Approximated for small values
    let amount = principal;
    if (time >= 1) amount = amount * (1 + rate);
    if (time >= 2) amount = amount * (1 + rate);
    if (time >= 3) amount = amount * (1 + rate);
    return amount;
}

let principal = 1000;
let rate = 0.05; // 5%
print "Investment of $" + principal + " at 5% for 3 years:";
print "Final amount: $" + compound_interest(principal, rate, 3);
print "";

function distance_formula(x1, y1, x2, y2) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    return sqrt(dx * dx + dy * dy);
}

print "Distance between (0,0) and (3,4): " + distance_formula(0, 0, 3, 4);
print "Distance between (1,1) and (4,5): " + distance_formula(1, 1, 4, 5);
print "";

print "=== Math Playground Complete! ===";
print "";
print "🎓 Topics covered:";
print "• Basic arithmetic operations";
print "• Built-in mathematical functions";
print "• Geometry (circles)";
print "• Number properties and patterns";
print "• Simple statistics with arrays";
print "• Factorials (small numbers)";
print "• Mathematical constants";
print "• Practical calculations";
print "";
print "💡 This version avoids complex loops for stability!";
print "Try the arrays demo for more advanced features!";
