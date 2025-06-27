// Functions Demo for Custom Language
print "=== Custom Language Functions Demo ===";

// 1. Simple Functions
print "";
print "1. Simple Functions:";

function greet(name) {
    return "Hello, " + name + "!";
}

function add(x, y) {
    return x + y;
}

function multiply(x, y) {
    return x * y;
}

print greet("Alice");
print greet("Bob");
print "5 + 3 = " + add(5, 3);
print "4 * 7 = " + multiply(4, 7);

// 2. Functions with Local Variables
print "";
print "2. Functions with Local Variables:";

function calculateArea(radius) {
    let pi = 3.14159;
    let area = pi * radius * radius;
    return area;
}

function calculateVolume(length, width, height) {
    let area = length * width;
    let volume = area * height;
    return volume;
}

print "Circle area (radius 5): " + calculateArea(5);
print "Box volume (2x3x4): " + calculateVolume(2, 3, 4);

// 3. Recursive Functions
print "";
print "3. Recursive Functions:";

function factorial(n) {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

function fibonacci(n) {
    if (n <= 1) {
        return n;
    } else {
        return fibonacci(n - 1) + fibonacci(n - 2);
    }
}

function power(base, exp) {
    if (exp == 0) {
        return 1;
    } else if (exp == 1) {
        return base;
    } else {
        return base * power(base, exp - 1);
    }
}

print "Factorial of 5: " + factorial(5);
print "Factorial of 6: " + factorial(6);
print "Fibonacci of 8: " + fibonacci(8);
print "Fibonacci of 10: " + fibonacci(10);
print "2^8 = " + power(2, 8);
print "3^4 = " + power(3, 4);

// 4. Functions Calling Other Functions
print "";
print "4. Functions Calling Other Functions:";

function isEven(n) {
    return n % 2 == 0;
}

function isOdd(n) {
    return !isEven(n);
}

function sumOfSquares(a, b) {
    return power(a, 2) + power(b, 2);
}

function distance(x1, y1, x2, y2) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    return sqrt(sumOfSquares(dx, dy));
}

print "Is 4 even? " + isEven(4);
print "Is 7 odd? " + isOdd(7);
print "Sum of squares of 3 and 4: " + sumOfSquares(3, 4);
print "Distance from (0,0) to (3,4): " + distance(0, 0, 3, 4);

print "";
print "Functions demo completed successfully!";
