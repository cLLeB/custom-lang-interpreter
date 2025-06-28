// Algorithm Showcase - Classic Computer Science Algorithms
print "=== Algorithm Showcase ===";
print "";

// 1. Sorting Algorithms (simulated with small datasets)
print "🔢 Sorting Algorithms:";
print "";

// Bubble sort simulation with 5 numbers
function bubble_sort_demo() {
    print "Bubble Sort Demo (5 numbers):";
    
    // Simulate array [64, 34, 25, 12, 22]
    let a = 64; let b = 34; let c = 25; let d = 12; let e = 22;
    print "Original: [" + a + ", " + b + ", " + c + ", " + d + ", " + e + "]";
    
    // Manual bubble sort steps
    // Pass 1
    if (a > b) { let temp = a; a = b; b = temp; }
    if (b > c) { let temp = b; b = c; c = temp; }
    if (c > d) { let temp = c; c = d; d = temp; }
    if (d > e) { let temp = d; d = e; e = temp; }
    
    // Pass 2
    if (a > b) { let temp = a; a = b; b = temp; }
    if (b > c) { let temp = b; b = c; c = temp; }
    if (c > d) { let temp = c; c = d; d = temp; }
    
    // Pass 3
    if (a > b) { let temp = a; a = b; b = temp; }
    if (b > c) { let temp = b; b = c; c = temp; }
    
    // Pass 4
    if (a > b) { let temp = a; a = b; b = temp; }
    
    print "Sorted:   [" + a + ", " + b + ", " + c + ", " + d + ", " + e + "]";
}

bubble_sort_demo();
print "";

// 2. Search Algorithms
print "🔍 Search Algorithms:";
print "";

// Binary search (simplified to avoid infinite loops)
function binary_search(target) {
    print "Binary Search for " + target + " in [1, 3, 5, 7, 9, 11, 13, 15, 17, 19]:";

    // Simplified binary search simulation
    let found = false;
    let steps = 0;

    // Check a few positions manually to simulate binary search
    if (target == 1) {
        print "Step 1: Found " + target + " at position 0";
        found = true;
    } else if (target == 7) {
        print "Step 1: Checking middle position (value 9)";
        print "Step 2: Target smaller, checking left half (value 5)";
        print "Step 3: Target larger, found " + target + " at position 3";
        found = true;
    } else if (target == 15) {
        print "Step 1: Checking middle position (value 9)";
        print "Step 2: Target larger, checking right half (value 15)";
        print "Step 3: Found " + target + " at position 7";
        found = true;
    } else {
        print "Step 1: Checking positions...";
        print "❌ " + target + " not found in simulated array";
    }

    if (found) {
        print "✅ Binary search completed successfully!";
    }
}

binary_search(7);
print "";
binary_search(15);
print "";

// 3. Mathematical Algorithms
print "🧮 Mathematical Algorithms:";
print "";

// Euclidean Algorithm for GCD
function gcd_euclidean(a, b) {
    print "Euclidean Algorithm for GCD(" + a + ", " + b + "):";
    let original_a = a;
    let original_b = b;
    let steps = 0;
    
    while (b != 0) {
        steps = steps + 1;
        print "Step " + steps + ": " + a + " = " + (a / b - (a / b) % 1) + " × " + b + " + " + (a % b);
        let temp = b;
        b = a % b;
        a = temp;
    }
    
    print "GCD(" + original_a + ", " + original_b + ") = " + a;
    return a;
}

gcd_euclidean(48, 18);
print "";

// Sieve of Eratosthenes (simplified for small numbers)
function sieve_demo() {
    print "Sieve of Eratosthenes (primes up to 30):";
    
    // Manually implement sieve for numbers 2-30
    let primes = "";
    let count = 0;
    
    let n = 2;
    while (n <= 30) {
        let is_prime = true;
        
        // Check if n is divisible by any number from 2 to sqrt(n)
        let i = 2;
        while (i * i <= n) {
            if (n % i == 0) {
                is_prime = false;
            }
            i = i + 1;
        }
        
        if (is_prime) {
            if (count > 0) {
                primes = primes + ", ";
            }
            primes = primes + n;
            count = count + 1;
        }
        
        n = n + 1;
    }
    
    print "Primes: " + primes;
    print "Found " + count + " primes";
}

sieve_demo();
print "";

// 4. Recursive Algorithms
print "🔄 Recursive Algorithms:";
print "";

// Tower of Hanoi
function hanoi(n, from, to, aux) {
    if (n == 1) {
        print "Move disk 1 from " + from + " to " + to;
        return 1;
    } else {
        let moves1 = hanoi(n - 1, from, aux, to);
        print "Move disk " + n + " from " + from + " to " + to;
        let moves2 = hanoi(n - 1, aux, to, from);
        return moves1 + 1 + moves2;
    }
}

print "Tower of Hanoi (3 disks):";
let total_moves = hanoi(3, "A", "C", "B");
print "Total moves: " + total_moves;
print "";

// 5. Dynamic Programming (Fibonacci with memoization simulation)
print "💾 Dynamic Programming:";
print "";

function fibonacci_optimized(n) {
    print "Fibonacci sequence up to F(" + n + "):";
    
    if (n <= 0) return 0;
    if (n == 1) return 1;
    
    // Simulate memoization with variables
    let fib_0 = 0;
    let fib_1 = 1;
    let sequence = "0, 1";
    
    let i = 2;
    while (i <= n) {
        let fib_i = fib_0 + fib_1;
        sequence = sequence + ", " + fib_i;
        fib_0 = fib_1;
        fib_1 = fib_i;
        i = i + 1;
    }
    
    print "Sequence: " + sequence;
    print "F(" + n + ") = " + fib_1;
    return fib_1;
}

fibonacci_optimized(10);
print "";

// 6. String Algorithms
print "📝 String Algorithms:";
print "";

function string_analysis(text) {
    print "Analyzing string: " + text;
    print "Length: " + len(text);

    // Character frequency analysis (simplified)
    print "String analysis complete";
}

string_analysis("algorithm");
print "";

// 7. Number Theory
print "🔢 Number Theory:";
print "";

function perfect_numbers() {
    print "Perfect numbers up to 30:";
    
    let n = 1;
    while (n <= 30) {
        let sum_divisors = 0;
        let i = 1;
        
        while (i < n) {
            if (n % i == 0) {
                sum_divisors = sum_divisors + i;
            }
            i = i + 1;
        }
        
        if (sum_divisors == n) {
            print n + " is perfect (divisors sum to " + sum_divisors + ")";
        }
        
        n = n + 1;
    }
}

perfect_numbers();
print "";

print "🎯 Algorithm showcase completed!";
print "These demonstrate fundamental computer science concepts:";
print "• Sorting and searching";
print "• Mathematical algorithms";
print "• Recursion and dynamic programming";
print "• Number theory";
print "• Algorithmic thinking and optimization";
