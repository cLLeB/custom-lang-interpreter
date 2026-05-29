// === Objects/Maps Demo ===
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
let mixed_object = {string_prop: "Hello", number_prop: 42, boolean_prop: true, null_prop: null, array_prop: [1, 2, 3]};
print "Mixed object: " + mixed_object;
print "String property: " + mixed_object["string_prop"];
print "Number property: " + mixed_object["number_prop"];
print "Boolean property: " + mixed_object["boolean_prop"];
print "Null property: " + mixed_object["null_prop"];
print "Array property: " + mixed_object["array_prop"];
print "";

// 3. Nested objects
print "3. Nested Objects:";
let company = {name: "Tech Corp", address: {street: "123 Main St", city: "San Francisco", zip: "94105"}, employees: 150};
print "Company: " + company;
print "Company name: " + company["name"];
print "Address: " + company["address"];
print "Street: " + company["address"]["street"];
print "City: " + company["address"]["city"];
print "Employee count: " + company["employees"];
print "";

// 4. Empty objects
print "4. Empty Objects:";
let empty_obj = {};
print "Empty object: " + empty_obj;
print "Type: " + get_type(empty_obj);
print "Accessing non-existent property: " + empty_obj["missing"];
print "";

// 5. Objects as function parameters
print "5. Objects as Function Parameters:";
function describe_person(person_obj) {
    let name = person_obj["name"];
    let age = person_obj["age"];
    let city = person_obj["city"];
    return name + " is " + age + " years old and lives in " + city;
}

let alice = {name: "Alice Smith", age: 28, city: "Boston"};
let bob = {name: "Bob Johnson", age: 35, city: "Seattle"};

print describe_person(alice);
print describe_person(bob);
print "";

// 6. Configuration objects
print "6. Configuration Objects:";
let config = {debug: true, max_retries: 3, timeout: 5000, api_url: "https://api.example.com", features: {logging: true, caching: false, analytics: true}};

print "Configuration: " + config;
print "Debug mode: " + config["debug"];
print "Max retries: " + config["max_retries"];
print "API URL: " + config["api_url"];
print "Logging enabled: " + config["features"]["logging"];
print "Caching enabled: " + config["features"]["caching"];
print "";

// 7. Data modeling
print "7. Data Modeling:";
let product = {id: 1001, name: "Laptop", price: 999.99, category: "Electronics", specs: {cpu: "Intel i7", ram: "16GB", storage: "512GB SSD"}, tags: ["computer", "portable", "work"]};

print "Product: " + product;
print "Product ID: " + product["id"];
print "Product name: " + product["name"];
print "Price: $" + product["price"];
print "CPU: " + product["specs"]["cpu"];
print "RAM: " + product["specs"]["ram"];
print "Tags: " + product["tags"];
print "";

// 8. User profiles
print "8. User Profiles:";
function create_user_profile(name, email, age) {
    return {name: name, email: email, age: age, created_at: "2024-01-01", preferences: {theme: "dark", notifications: true, language: "en"}};
}

let user1 = create_user_profile("Charlie Brown", "charlie@example.com", 25);
let user2 = create_user_profile("Diana Prince", "diana@example.com", 32);

print "User 1: " + user1;
print "User 1 email: " + user1["email"];
print "User 1 theme: " + user1["preferences"]["theme"];

print "User 2: " + user2;
print "User 2 name: " + user2["name"];
print "User 2 notifications: " + user2["preferences"]["notifications"];
print "";

// 9. API response simulation
print "9. API Response Simulation:";
let api_response = {status: "success", code: 200, data: {users: [{name: "User1", active: true}, {name: "User2", active: false}], total: 2}, message: "Users retrieved successfully"};

print "API Response: " + api_response;
print "Status: " + api_response["status"];
print "Code: " + api_response["code"];
print "Message: " + api_response["message"];
print "Total users: " + api_response["data"]["total"];
print "First user: " + api_response["data"]["users"][0];
print "First user name: " + api_response["data"]["users"][0]["name"];
print "";

print "=== Objects Demo Complete! ===";
print "";
print "Object Features:";
print "- Object literal syntax: {key: value}";
print "- Property access: obj[key]";
print "- Nested objects and arrays";
print "- Mixed value types";
print "- Objects as function parameters";
print "- Dynamic property access";
print "";
print "Use Cases:";
print "- Configuration management";
print "- Data modeling and structures";
print "- API response handling";
print "- User profiles and settings";
print "- Complex data organization";
print "";
print "Coming Soon:";
print "- Dot notation: obj.property";
print "- Object methods and functions";
print "- Property assignment and modification";
print "- Object iteration and manipulation";
