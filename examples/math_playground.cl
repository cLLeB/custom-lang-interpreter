// Math Playground - Exploring Mathematical Concepts
print "=== Math Playground ===";
print "";

// 1. Number Sequences
print "🔢 Famous Number Sequences:";
print "";

// Simple Fibonacci sequence (no loops)
function fibonacci_simple() {
    print "Fibonacci sequence (first 10 terms):";
    let f0 = 0;
    let f1 = 1;
    let f2 = f0 + f1;
    let f3 = f1 + f2;
    let f4 = f2 + f3;
    let f5 = f3 + f4;
    let f6 = f4 + f5;
    let f7 = f5 + f6;
    let f8 = f6 + f7;
    let f9 = f7 + f8;

    print "0, 1, " + f2 + ", " + f3 + ", " + f4 + ", " + f5 + ", " + f6 + ", " + f7 + ", " + f8 + ", " + f9;
}

fibonacci_simple();
print "";

// Simple prime check
function is_prime_simple(n) {
    if (n <= 1) return false;
    if (n <= 3) return true;
    if (n % 2 == 0) return false;
    if (n % 3 == 0) return false;
    return true;  // Simplified check
}

print "Prime checking:";
print "Is 2 prime? " + is_prime_simple(2);
print "Is 7 prime? " + is_prime_simple(7);
print "Is 9 prime? " + is_prime_simple(9);
print "Is 17 prime? " + is_prime_simple(17);
print "";

// 2. Geometric Calculations
print "📐 Geometry:";
print "";

function circle_properties(radius) {
    let pi = 3.14159;
    let area = pi * radius * radius;
    let circumference = 2 * pi * radius;

    print "Circle with radius " + radius + ":";
    print "  Area: " + area;
    print "  Circumference: " + circumference;
    print "  Diameter: " + (2 * radius);
}

circle_properties(5);
print "";

function triangle_properties(a, b, c) {
    print "Triangle with sides " + a + ", " + b + ", " + c + ":";

    // Check if valid triangle
    if (a + b > c && a + c > b && b + c > a) {
        let perimeter = a + b + c;
        print "  Perimeter: " + perimeter;

        // Triangle type
        if (a == b && b == c) {
            print "  Type: Equilateral";
        } else if (a == b || b == c || a == c) {
            print "  Type: Isosceles";
        } else {
            print "  Type: Scalene";
        }

        // Check if right triangle (3-4-5 case)
        if (a == 3 && b == 4 && c == 5) {
            print "  Special: Right triangle";
        }
    } else {
        print "  Invalid triangle (triangle inequality violated)";
    }
}

triangle_properties(3, 4, 5);
print "";
triangle_properties(5, 5, 5);
print "";

// 3. Number Theory Fun
print "🧮 Number Theory:";
print "";

function number_properties(n) {
    print "Properties of " + n + ":";

    // Basic properties
    print "  Even: " + (n % 2 == 0);
    print "  Odd: " + (n % 2 == 1);

    // Divisibility tests
    print "  Divisible by 3: " + (n % 3 == 0);
    print "  Divisible by 5: " + (n % 5 == 0);
    print "  Divisible by 7: " + (n % 7 == 0);

    // Perfect square check
    let sqrt_n = sqrt(n);
    let is_perfect_square = (sqrt_n * sqrt_n == n);
    print "  Perfect square: " + is_perfect_square;

    // Simple factorial for small numbers
    if (n == 5) {
        print "  Factorial: " + (1 * 2 * 3 * 4 * 5);
    } else if (n == 6) {
        print "  Factorial: " + (1 * 2 * 3 * 4 * 5 * 6);
    }
}

number_properties(24);
print "";
number_properties(49);
print "";

// 4. Mathematical Series
print "📊 Mathematical Series:";
print "";

function arithmetic_series(first, diff, terms) {
    print "Arithmetic series: first=" + first + ", difference=" + diff + ", terms=" + terms;
    
    let sum = 0;
    let current = first;
    let series = "" + first;
    
    let i = 1;
    while (i < terms) {
        current = current + diff;
        series = series + ", " + current;
        sum = sum + current;
        i = i + 1;
    }
    
    sum = sum + first;  // Add the first term
    let formula_sum = terms * (2 * first + (terms - 1) * diff) / 2;
    
    print "  Series: " + series;
    print "  Sum: " + sum;
    print "  Formula check: " + formula_sum;
}

arithmetic_series(2, 3, 8);
print "";

function geometric_series(first, ratio, terms) {
    print "Geometric series: first=" + first + ", ratio=" + ratio + ", terms=" + terms;
    
    let sum = 0;
    let current = first;
    let series = "" + first;
    
    let i = 1;
    while (i < terms) {
        current = current * ratio;
        series = series + ", " + current;
        sum = sum + current;
        i = i + 1;
    }
    
    sum = sum + first;  // Add the first term
    
    print "  Series: " + series;
    print "  Sum: " + sum;
}

geometric_series(2, 3, 6);
print "";

// 5. Combinatorics
print "🎲 Combinatorics:";
print "";

function factorial(n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

function combinations(n, r) {
    if (r > n) return 0;
    if (r == 0 || r == n) return 1;
    
    let numerator = factorial(n);
    let denominator = factorial(r) * factorial(n - r);
    return numerator / denominator;
}

function permutations(n, r) {
    if (r > n) return 0;
    let result = factorial(n) / factorial(n - r);
    return result;
}

print "Combinatorics examples:";
print "C(5,2) = " + combinations(5, 2) + " (ways to choose 2 from 5)";
print "P(5,2) = " + permutations(5, 2) + " (ways to arrange 2 from 5)";
print "C(8,3) = " + combinations(8, 3) + " (ways to choose 3 from 8)";
print "";

// 6. Mathematical Constants and Approximations
print "🔬 Mathematical Constants:";
print "";

function pi_approximation() {
    // Leibniz formula for π/4
    let pi_over_4 = 0;
    let sign = 1;
    let i = 0;
    
    while (i < 1000) {
        pi_over_4 = pi_over_4 + sign / (2 * i + 1);
        sign = sign * (-1);
        i = i + 1;
    }
    
    let pi_approx = 4 * pi_over_4;
    print "π approximation (Leibniz series): " + pi_approx;
    print "Built-in comparison: 3.14159";
}

pi_approximation();
print "";

function e_approximation() {
    // e = sum(1/n!) for n=0 to infinity
    let e_approx = 1;  // 1/0! = 1
    let factorial_n = 1;
    
    let n = 1;
    while (n <= 15) {
        factorial_n = factorial_n * n;
        e_approx = e_approx + (1 / factorial_n);
        n = n + 1;
    }
    
    print "e approximation (Taylor series): " + e_approx;
}

e_approximation();
print "";

// 7. Fun Math Puzzles
print "🧩 Math Puzzles:";
print "";

function collatz_sequence(n) {
    print "Collatz sequence starting from " + n + ":";
    let sequence = "" + n;
    let steps = 0;
    
    while (n != 1) {
        if (n % 2 == 0) {
            n = n / 2;
        } else {
            n = 3 * n + 1;
        }
        sequence = sequence + " → " + n;
        steps = steps + 1;
        
        if (steps > 20) {  // Limit output length
            sequence = sequence + " → ...";
            n = 1;  // Force exit condition
        }
    }
    
    print sequence;
    print "Steps to reach 1: " + steps;
}

collatz_sequence(7);
print "";

print "🎯 Math playground exploration complete!";
print "Mathematics is everywhere - from simple arithmetic to complex patterns!";
print "Keep exploring and discovering the beauty of numbers! 🌟";
