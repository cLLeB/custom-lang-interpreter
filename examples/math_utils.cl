// Math utilities module
print "Loading math_utils module...";

// Basic math functions
function add(a, b) {
    return a + b;
}

function multiply(a, b) {
    return a * b;
}

function square(x) {
    return x * x;
}

// Constants
let PI = 3.14159;
let E = 2.71828;

// Export functions and constants
export add;
export multiply;
export square;
export PI;
export E;

print "Math utils module loaded!";
