// Test pattern matching
print "=== Pattern Matching Demo ===";
print "";

// 1. Basic literal matching
print "1. Literal Pattern Matching:";
let value1 = 42;
let result1 = match value1 {
    42 => "Found the answer!",
    0 => "Zero",
    _ => "Something else"
};
print "match 42: " + result1;

let value2 = "hello";
let result2 = match value2 {
    "hello" => "Greeting",
    "goodbye" => "Farewell", 
    _ => "Unknown"
};
print "match 'hello': " + result2;

print "";

// 2. Variable binding
print "2. Variable Binding:";
let value3 = 100;
let result3 = match value3 {
    x => "Captured value: " + x
};
print result3;

print "";

// 3. Array pattern matching
print "3. Array Pattern Matching:";
let arr = [1, 2, 3];
let result4 = match arr {
    [1, 2, 3] => "Exact match: [1, 2, 3]",
    [a, b, c] => "Three elements: " + a + ", " + b + ", " + c,
    _ => "Different pattern"
};
print result4;

print "";

// 4. Object pattern matching
print "4. Object Pattern Matching:";
let obj = {name: "Alice", age: 30};
let result5 = match obj {
    {name: "Alice", age: a} => "Alice is " + a + " years old",
    {name: n, age: _} => "Person named " + n,
    _ => "Unknown object"
};
print result5;

print "";

// 5. Wildcard patterns
print "5. Wildcard Patterns:";
let value6 = true;
let result6 = match value6 {
    true => "It's true!",
    _ => "It's something else"
};
print result6;

print "";
print "=== Pattern Matching Demo Complete! ===";
print "";
print "Pattern Matching Features Demonstrated:";
print "- Literal patterns (numbers, strings, booleans)";
print "- Variable binding patterns";
print "- Array destructuring patterns";
print "- Object destructuring patterns";
print "- Wildcard patterns (_)";
print "- Match expressions with multiple arms";
