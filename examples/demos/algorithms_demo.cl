// Algorithms Demo for Custom Language
print "=== Custom Language Algorithms Demo ===";

// 1. Sorting Algorithm - Bubble Sort
print "";
print "1. Bubble Sort Algorithm:";

function bubbleSort(arr, size) {
    let i = 0;
    while (i < size - 1) {
        let j = 0;
        while (j < size - i - 1) {
            // For demo purposes, we'll just show the concept
            // In a real implementation, we'd need array support
            j = j + 1;
        }
        i = i + 1;
    }
    print "Bubble sort algorithm demonstrated (arrays not fully implemented)";
}

bubbleSort(null, 5);

// 2. Mathematical Algorithms
print "\n2. Mathematical Algorithms:";

// Greatest Common Divisor (Euclidean Algorithm)
function gcd(a, b) {
    while (b != 0) {
        let temp = b;
        b = a % b;
        a = temp;
    }
    return a;
}

// Least Common Multiple
function lcm(a, b) {
    return (a * b) / gcd(a, b);
}

// Check if a number is prime
function isPrime(n) {
    if (n <= 1) {
        return false;
    }
    if (n <= 3) {
        return true;
    }
    if (n % 2 == 0 || n % 3 == 0) {
        return false;
    }
    
    let i = 5;
    while (i * i <= n) {
        if (n % i == 0 || n % (i + 2) == 0) {
            return false;
        }
        i = i + 6;
    }
    return true;
}

// Generate nth prime number
function nthPrime(n) {
    let count = 0;
    let num = 2;
    
    while (count < n) {
        if (isPrime(num)) {
            count = count + 1;
            if (count == n) {
                return num;
            }
        }
        num = num + 1;
    }
    return num;
}

print "GCD of 48 and 18: " + gcd(48, 18);
print "LCM of 12 and 8: " + lcm(12, 8);
print "Is 17 prime? " + isPrime(17);
print "Is 15 prime? " + isPrime(15);
print "5th prime number: " + nthPrime(5);
print "10th prime number: " + nthPrime(10);

// 3. Number Theory
print "\n3. Number Theory:";

// Sum of digits
function sumOfDigits(n) {
    let sum = 0;
    while (n > 0) {
        sum = sum + (n % 10);
        n = (n - (n % 10)) / 10;
    }
    return sum;
}

// Reverse a number
function reverseNumber(n) {
    let reversed = 0;
    while (n > 0) {
        reversed = reversed * 10 + (n % 10);
        n = (n - (n % 10)) / 10;
    }
    return reversed;
}

// Check if a number is palindrome
function isPalindrome(n) {
    return n == reverseNumber(n);
}

// Perfect number check
function isPerfect(n) {
    let sum = 1;
    let i = 2;
    while (i * i <= n) {
        if (n % i == 0) {
            sum = sum + i;
            if (i * i != n) {
                sum = sum + (n / i);
            }
        }
        i = i + 1;
    }
    return sum == n && n > 1;
}

print "Sum of digits of 12345: " + sumOfDigits(12345);
print "Reverse of 12345: " + reverseNumber(12345);
print "Is 12321 a palindrome? " + isPalindrome(12321);
print "Is 12345 a palindrome? " + isPalindrome(12345);
print "Is 6 a perfect number? " + isPerfect(6);
print "Is 28 a perfect number? " + isPerfect(28);

// 4. Recursive Algorithms
print "\n4. Recursive Algorithms:";

// Tower of Hanoi (just count moves)
function hanoi(n) {
    if (n == 1) {
        return 1;
    } else {
        return 2 * hanoi(n - 1) + 1;
    }
}

// Ackermann function (small values only)
function ackermann(m, n) {
    if (m == 0) {
        return n + 1;
    } else if (n == 0) {
        return ackermann(m - 1, 1);
    } else {
        return ackermann(m - 1, ackermann(m, n - 1));
    }
}

print "Hanoi moves for 3 disks: " + hanoi(3);
print "Hanoi moves for 4 disks: " + hanoi(4);
print "Ackermann(2, 3): " + ackermann(2, 3);
print "Ackermann(3, 2): " + ackermann(3, 2);

// 5. Iterative vs Recursive Comparison
print "\n5. Iterative vs Recursive Comparison:";

// Iterative factorial
function factorialIterative(n) {
    let result = 1;
    let i = 1;
    while (i <= n) {
        result = result * i;
        i = i + 1;
    }
    return result;
}

// Iterative fibonacci
function fibonacciIterative(n) {
    if (n <= 1) {
        return n;
    }
    
    let a = 0;
    let b = 1;
    let i = 2;
    
    while (i <= n) {
        let temp = a + b;
        a = b;
        b = temp;
        i = i + 1;
    }
    return b;
}

print "Factorial 6 (iterative): " + factorialIterative(6);
print "Factorial 6 (recursive): " + factorial(6);
print "Fibonacci 10 (iterative): " + fibonacciIterative(10);
print "Fibonacci 10 (recursive): " + fibonacci(10);

print "\nAlgorithms demo completed successfully!";
