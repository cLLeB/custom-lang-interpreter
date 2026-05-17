// Showcase of all new features added in v1.0.0
print "=== New Features Demo v1.0.0 ===";
print "";

// 1. For loops (C-style and for-in)
print "1. For Loops:";
let sum = 0;
for (let i = 1; i <= 5; i += 1) {
    sum += i;
}
print "Sum 1-5 = " + sum;

let arr = [10, 20, 30, 40, 50];
let total = 0;
for (item in arr) {
    total += item;
}
print "Array total = " + total;
print "";

// 2. Break and Continue
print "2. Break and Continue:";
let found = -1;
for (let i = 0; i < 100; i += 1) {
    if (i * i > 50) {
        found = i;
        break;
    }
}
print "First i where i^2 > 50: " + found;

let evens = 0;
for (let i = 1; i <= 20; i += 1) {
    if (i % 2 != 0) { continue; }
    evens += i;
}
print "Sum of evens 1-20: " + evens;
print "";

// 3. Lambdas and higher-order functions
print "3. Lambdas and Higher-Order Functions:";
let nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

let odds = filter(nums, function(x) { return x % 2 != 0; });
print "Odd numbers: " + odds;

let squares = map(nums, function(x) { return x * x; });
print "Squares: " + squares;

let product = reduce([1, 2, 3, 4, 5], function(acc, x) { return acc * x; }, 1);
print "Product of 1-5: " + product;
print "";

// 4. Closures
print "4. Closures:";
function make_counter(start) {
    let count = start;
    return function() {
        count += 1;
        return count;
    };
}

let counter = make_counter(0);
print "Counter: " + counter() + ", " + counter() + ", " + counter();

function make_multiplier(factor) {
    return function(x) { return x * factor; };
}
let triple = make_multiplier(3);
print "Triple 7 = " + triple(7);
print "";

// 5. Pattern matching
print "5. Pattern Matching:";
function classify(x) {
    return match x {
        0 => "zero",
        1 => "one",
        n => "other: " + n
    };
}
print classify(0);
print classify(1);
print classify(42);

function describe_list(lst) {
    return match lst {
        [] => "empty",
        [x] => "single: " + x,
        [a, b] => "pair: " + a + " and " + b,
        _ => "many elements"
    };
}
print describe_list([]);
print describe_list([42]);
print describe_list([1, 2]);
print describe_list([1, 2, 3]);
print "";

// 6. Classes with this and property compound assign
print "6. Classes:";
class BankAccount {
    function init(owner, balance) {
        this.owner = owner;
        this.balance = balance;
    }
    function deposit(amount) {
        this.balance += amount;
    }
    function withdraw(amount) {
        if (amount > this.balance) {
            print "Insufficient funds!";
            return;
        }
        this.balance -= amount;
    }
    function status() {
        print this.owner + "'s balance: " + this.balance;
    }
}

let account = new BankAccount("Alice", 1000);
account.status();
account.deposit(500);
account.status();
account.withdraw(200);
account.status();
account.withdraw(2000);
print "";

// 7. String escape sequences
print "7. String Escapes:";
print "Tab:\there";
print "Newline in string literal: \\n";
let quoted = "She said \"hello\"";
print quoted;
print "";

// 8. Object manipulation
print "8. Object Built-ins:";
let person = {name: "Bob", age: 25, city: "Accra"};
print "Keys: " + keys(person);
print "Values: " + values(person);
print "Has 'name': " + has_key(person, "name");
print "Has 'phone': " + has_key(person, "phone");
delete_key(person, "city");
print "After delete city: " + keys(person);
print "";

print "=== Demo Complete! ===";
