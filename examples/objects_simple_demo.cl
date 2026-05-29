// === Objects/Maps Demo (Simple Version) ===
// Demonstration of object literal syntax and manipulation

print "=== Objects/Maps Demo ===";
print "";

// 1. Basic object creation
print "1. Basic Object Creation:";
let person = {name: "John Doe", age: 30, city: "New York"};
print "Person object: " + person;
print "Name: " + person["name"];
print "Age: " + person["age"];
print "City: " + person["city"];
print "";

// 2. Objects with different value types
print "2. Mixed Value Types:";
let mixed = {str: "Hello", num: 42, bool: true, null_val: null};
print "Mixed object: " + mixed;
print "String: " + mixed["str"];
print "Number: " + mixed["num"];
print "Boolean: " + mixed["bool"];
print "Null: " + mixed["null_val"];
print "";

// 3. Empty objects
print "3. Empty Objects:";
let empty_obj = {};
print "Empty object: " + empty_obj;
print "Type: " + get_type(empty_obj);
print "Missing property: " + empty_obj["missing"];
print "";

// 4. Objects as function parameters
print "4. Objects as Function Parameters:";
function describe_person(p) {
    return p["name"] + " is " + p["age"] + " years old";
}

let alice = {name: "Alice", age: 28};
let bob = {name: "Bob", age: 35};

print describe_person(alice);
print describe_person(bob);
print "";

// 5. Configuration objects
print "5. Configuration Objects:";
let config = {debug: true, retries: 3, timeout: 5000};
print "Config: " + config;
print "Debug: " + config["debug"];
print "Retries: " + config["retries"];
print "Timeout: " + config["timeout"];
print "";

// 6. Product catalog
print "6. Product Catalog:";
let product1 = {id: 1001, name: "Laptop", price: 999.99, category: "Electronics"};
let product2 = {id: 1002, name: "Mouse", price: 29.99, category: "Accessories"};

print "Product 1: " + product1;
print "Product 1 name: " + product1["name"];
print "Product 1 price: $" + product1["price"];

print "Product 2: " + product2;
print "Product 2 name: " + product2["name"];
print "Product 2 price: $" + product2["price"];
print "";

// 7. User profiles
print "7. User Profiles:";
function create_user(name, email, age) {
    return {name: name, email: email, age: age, active: true};
}

let user1 = create_user("Charlie", "charlie@example.com", 25);
let user2 = create_user("Diana", "diana@example.com", 32);

print "User 1: " + user1;
print "User 1 email: " + user1["email"];
print "User 1 active: " + user1["active"];

print "User 2: " + user2;
print "User 2 name: " + user2["name"];
print "User 2 age: " + user2["age"];
print "";

// 8. Simple nested objects
print "8. Simple Nested Objects:";
let company = {name: "Tech Corp", employees: 150};
let address = {street: "123 Main St", city: "SF", zip: "94105"};

print "Company: " + company;
print "Address: " + address;
print "Company name: " + company["name"];
print "Address city: " + address["city"];
print "";

// 9. Object comparison and manipulation
print "9. Object Operations:";
let obj1 = {a: 1, b: 2};
let obj2 = {x: 10, y: 20};

print "Object 1: " + obj1;
print "Object 2: " + obj2;
print "obj1 type: " + get_type(obj1);
print "obj2 type: " + get_type(obj2);

// Test property access
print "obj1[a]: " + obj1["a"];
print "obj2[x]: " + obj2["x"];
print "obj1[missing]: " + obj1["missing"];
print "";

// 10. Objects with arrays
print "10. Objects with Arrays:";
let data = {numbers: [1, 2, 3], strings: ["a", "b", "c"]};
print "Data object: " + data;
print "Numbers array: " + data["numbers"];
print "Strings array: " + data["strings"];
print "First number: " + data["numbers"][0];
print "Last string: " + data["strings"][2];
print "";

print "=== Objects Demo Complete! ===";
print "";
print "Object Features Demonstrated:";
print "- Object literal syntax: {key: value}";
print "- Property access with brackets: obj[key]";
print "- Mixed value types in objects";
print "- Empty objects";
print "- Objects as function parameters and return values";
print "- Configuration and data modeling";
print "- Property access with missing keys (returns null)";
print "";
print "Use Cases:";
print "- Configuration management";
print "- Data structures and modeling";
print "- User profiles and settings";
print "- Product catalogs";
print "- API data representation";
print "";
print "Note: Multi-line object literals coming soon!";
