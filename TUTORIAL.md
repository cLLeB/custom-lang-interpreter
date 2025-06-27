# Custom Language Tutorial

Welcome to the Custom Language tutorial! This guide will walk you through all the features of our programming language.

## Getting Started

### Running Your First Program

Create a file called `hello.cl`:
```javascript
print "Hello, World!";
```

Run it:
```bash
cargo run -- hello.cl
```

### Using the REPL

Start the interactive mode:
```bash
cargo run -- --repl
```

Try these commands:
```javascript
> print "Hello from REPL!"
> let x = 42
> print x
> exit
```

## Variables and Data Types

### Numbers
```javascript
let integer = 42;
let decimal = 3.14159;
let negative = -17;

print integer + decimal;  // 45.14159
print negative * 2;       // -34
```

### Strings
```javascript
let greeting = "Hello";
let name = "Alice";
let message = greeting + ", " + name + "!";

print message;            // Hello, Alice!
print len(message);       // 13
```

### Booleans
```javascript
let isTrue = true;
let isFalse = false;

print isTrue && isFalse;  // false
print isTrue || isFalse;  // true
print !isTrue;            // false
```

### Null
```javascript
let nothing = null;
print nothing;            // null
print type(nothing);      // "null"
```

## Operators

### Arithmetic
```javascript
let a = 10;
let b = 3;

print a + b;    // 13
print a - b;    // 7
print a * b;    // 30
print a / b;    // 3.333...
print a % b;    // 1
```

### Comparison
```javascript
let x = 5;
let y = 10;

print x == y;   // false
print x != y;   // true
print x < y;    // true
print x <= y;   // true
print x > y;    // false
print x >= y;   // false
```

### Logical
```javascript
let p = true;
let q = false;

print p && q;   // false
print p || q;   // true
print !p;       // false
print !q;       // true
```

## Control Flow

### If/Else Statements
```javascript
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
```

### While Loops
```javascript
let count = 0;
while (count < 5) {
    print "Count: " + count;
    count = count + 1;
}
```

### Nested Control Flow
```javascript
let i = 1;
while (i <= 3) {
    let j = 1;
    while (j <= 3) {
        if (i == j) {
            print "Diagonal: " + i + "," + j;
        }
        j = j + 1;
    }
    i = i + 1;
}
```

## Functions

### Basic Functions
```javascript
function greet(name) {
    return "Hello, " + name + "!";
}

let message = greet("Bob");
print message;  // Hello, Bob!
```

### Functions with Multiple Parameters
```javascript
function add(x, y) {
    return x + y;
}

function multiply(x, y) {
    return x * y;
}

let sum = add(5, 3);
let product = multiply(4, 7);
print "Sum: " + sum;        // Sum: 8
print "Product: " + product; // Product: 28
```

### Functions with Local Variables
```javascript
function calculateCircleArea(radius) {
    let pi = 3.14159;
    let area = pi * radius * radius;
    return area;
}

print calculateCircleArea(5);  // 78.53975
```

### Recursive Functions
```javascript
function factorial(n) {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

print factorial(5);  // 120
print factorial(0);  // 1
```

### Advanced Recursion
```javascript
function fibonacci(n) {
    if (n <= 1) {
        return n;
    } else {
        return fibonacci(n - 1) + fibonacci(n - 2);
    }
}

// Print first 10 Fibonacci numbers
let i = 0;
while (i < 10) {
    print "fib(" + i + ") = " + fibonacci(i);
    i = i + 1;
}
```

## Built-in Functions

### Math Functions
```javascript
print abs(-42);      // 42
print sqrt(16);      // 4
print pow(2, 8);     // 256
print min(10, 5);    // 5
print max(10, 5);    // 10
```

### String Functions
```javascript
let text = "Programming";
print len(text);     // 11
```

### Type Inspection
```javascript
print type(42);      // "number"
print type("text");  // "string"
print type(true);    // "boolean"
print type(null);    // "null"
```

## Advanced Examples

### Number Guessing Game
```javascript
function playGame() {
    let secret = 42;  // In a real game, this would be random
    let guess = 35;   // In a real game, this would be user input
    
    if (guess == secret) {
        print "Congratulations! You guessed it!";
    } else if (guess < secret) {
        print "Too low! Try again.";
    } else {
        print "Too high! Try again.";
    }
}

playGame();
```

### Mathematical Calculations
```javascript
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

// Test prime numbers
let num = 17;
if (isPrime(num)) {
    print num + " is prime";
} else {
    print num + " is not prime";
}
```

### Function Composition
```javascript
function square(x) {
    return x * x;
}

function double(x) {
    return x * 2;
}

function compose(x) {
    return double(square(x));
}

print compose(5);  // double(square(5)) = double(25) = 50
```

## Best Practices

### 1. Use Descriptive Names
```javascript
// Good
function calculateTotalPrice(price, taxRate) {
    return price * (1 + taxRate);
}

// Avoid
function calc(p, t) {
    return p * (1 + t);
}
```

### 2. Keep Functions Small
```javascript
// Good - single responsibility
function isEven(n) {
    return n % 2 == 0;
}

function printNumbers(start, end) {
    let i = start;
    while (i <= end) {
        if (isEven(i)) {
            print i + " is even";
        }
        i = i + 1;
    }
}
```

### 3. Handle Edge Cases
```javascript
function safeDivide(a, b) {
    if (b == 0) {
        print "Error: Division by zero";
        return null;
    }
    return a / b;
}
```

## Next Steps

1. **Experiment**: Try modifying the examples in this tutorial
2. **Build Projects**: Create your own programs using these concepts
3. **Explore Examples**: Check out the `examples/` directory for more code
4. **Use the REPL**: Perfect for testing small code snippets
5. **Read the Documentation**: See README.md for complete reference

Happy coding with Custom Language! 🚀
