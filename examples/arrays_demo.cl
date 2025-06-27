// Arrays Demo 
print "=== Arrays Demo (v0.2.0) ===";
print "";

// Creating arrays
print "1. Creating Arrays:";
let empty_array = [];
let numbers = [1, 2, 3, 4, 5];
let mixed = [42, "hello", true, null];
let nested = [[1, 2], [3, 4], [5, 6]];

print "Empty array: " + empty_array;
print "Numbers: " + numbers;
print "Mixed types: " + mixed;
print "Nested arrays: " + nested;
print "";

// Array indexing
print "2. Array Indexing:";
print "numbers[0] = " + numbers[0];
print "numbers[2] = " + numbers[2];
print "numbers[4] = " + numbers[4];
print "mixed[1] = " + mixed[1];
print "nested[0] = " + nested[0];
print "nested[1][0] = " + nested[1][0];
print "";

// Array length
print "3. Array Length:";
print "len(empty_array) = " + len(empty_array);
print "len(numbers) = " + len(numbers);
print "len(mixed) = " + len(mixed);
print "len(nested) = " + len(nested);
print "";

// Array functions
print "4. Array Functions:";
print "first(numbers) = " + first(numbers);
print "last(numbers) = " + last(numbers);
print "first(mixed) = " + first(mixed);
print "last(mixed) = " + last(mixed);
print "";

// Array manipulation
print "5. Array Manipulation:";
let original = [10, 20, 30];
print "Original: " + original;

let with_push = push(original, 40);
print "After push(40): " + with_push;

let popped = pop(with_push);
print "pop() result: " + popped;
print "";

// String indexing (bonus feature)
print "6. String Indexing:";
let text = "Hello";
print "text = " + text;
print "text[0] = " + text[0];
print "text[1] = " + text[1];
print "text[4] = " + text[4];
print "len(text) = " + len(text);
print "";

// Working with arrays in functions
print "7. Arrays in Functions:";

function sum_array(arr) {
    let total = 0;
    let i = 0;
    // Manual iteration since we don't have for loops yet
    if (len(arr) > 0) total = total + arr[0];
    if (len(arr) > 1) total = total + arr[1];
    if (len(arr) > 2) total = total + arr[2];
    if (len(arr) > 3) total = total + arr[3];
    if (len(arr) > 4) total = total + arr[4];
    return total;
}

function array_info(arr) {
    print "Array: " + arr;
    print "Length: " + len(arr);
    if (len(arr) > 0) {
        print "First: " + first(arr);
        print "Last: " + last(arr);
    }
}

let test_numbers = [5, 10, 15, 20];
print "sum_array([5, 10, 15, 20]) = " + sum_array(test_numbers);
print "";
array_info(test_numbers);
print "";

// Array comparison and type checking
print "8. Array Type Checking:";
print "type([1, 2, 3]) = " + type([1, 2, 3]);
print "type([]) = " + type([]);
print "type(numbers) = " + type(numbers);
print "";

// Complex array operations
print "9. Complex Operations:";

function create_range(start, end) {
    // Create array with numbers from start to end
    let result = [];
    if (start <= end) {
        result = push(result, start);
        if (start + 1 <= end) result = push(result, start + 1);
        if (start + 2 <= end) result = push(result, start + 2);
        if (start + 3 <= end) result = push(result, start + 3);
        if (start + 4 <= end) result = push(result, start + 4);
    }
    return result;
}

let range = create_range(1, 5);
print "create_range(1, 5) = " + range;
print "";

// Array truthiness
print "10. Array Truthiness:";
if (empty_array) {
    print "Empty array is truthy";
} else {
    print "Empty array is falsy";
}

if (numbers) {
    print "Non-empty array is truthy";
} else {
    print "Non-empty array is falsy";
}
print "";

print "=== Arrays Demo Complete! ===";
print "";
print "🎉 New features in v0.2.0:";
print "• Array literals: [1, 2, 3]";
print "• Array indexing: arr[0]";
print "• String indexing: str[0]";
print "• Array functions: push(), pop(), first(), last()";
print "• Enhanced len() for arrays";
print "• Mixed-type arrays";
print "• Nested arrays";
print "• Array type checking";
print "";

