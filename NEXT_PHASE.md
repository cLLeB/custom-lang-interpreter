# Custom Language — Next Phase Roadmap

Everything below is missing and should be implemented. Grouped by theme and roughly ordered by priority within each group. Pick up from here when ready.

---

## Phase 1 — Core Syntax Gaps (Highest Priority)

These are things every mainstream scripting language has. Without them, real programs are painful to write.

### 1.1 Try / Catch / Finally / Throw
```
try {
    let data = read_file("config.json");
} catch (e) {
    print "Error: " + e.message;
} finally {
    print "Always runs";
}

throw "something went wrong";
throw { code: 404, message: "not found" };
```
- Custom exception classes: `class MyError extends Error { ... }`
- Error chaining: `throw new NetworkError("timeout", cause: original_error)`
- Stack traces on errors

### 1.2 Ternary Operator
```
let label = x > 0 ? "positive" : "non-positive";
```

### 1.3 Null Coalescing
```
let name = user.name ?? "Anonymous";
let port = config.port ?? 8080;
```

### 1.4 Optional Chaining
```
let city = user?.address?.city;         // null if any step is null
let first = arr?.[0];                   // null if arr is null
user?.save();                           // no-op if user is null
```

### 1.5 Logical Assignment Operators
```
x ||= default_value;    // x = x || default_value
x &&= transform(x);     // x = x && transform(x)
x ??= fallback;         // x = x ?? fallback
```

### 1.6 Exponentiation Operator
```
let area = r ** 2 * PI;
let big = 2 ** 32;
```

### 1.7 Bitwise Operators
```
let flags = 0b1010 & 0b1100;   // AND
let mask  = 0b0001 | 0b0010;   // OR
let flip  = ~0b1010;           // NOT
let xor   = a ^ b;             // XOR
let left  = 1 << 4;            // shift left  = 16
let right = 256 >> 3;          // shift right = 32
let uright = -1 >>> 0;         // unsigned right shift
```
- Also hex literals `0xFF`, octal `0o77`, binary `0b1010`

### 1.8 `in` Operator
```
if ("name" in person) { ... }
if (42 in [1, 42, 99]) { ... }
```

### 1.9 `instanceof` / `is` Operator
```
if (animal instanceof Dog) { ... }
if (x is number) { ... }        // type check shorthand
```

### 1.10 Do-While Loop
```
do {
    let input = read_line();
} while (input != "quit");
```

### 1.11 Labeled Break / Continue (for nested loops)
```
outer: for (let i = 0; i < 5; i += 1) {
    for (let j = 0; j < 5; j += 1) {
        if (j == 3) break outer;
    }
}
```

### 1.12 String Interpolation / Template Literals
```
let msg = `Hello ${name}, you are ${age} years old`;
let multiline = `
    SELECT *
    FROM users
    WHERE id = ${user_id}
`;
```

### 1.13 Multi-line Strings / Heredoc
```
let sql = """
    SELECT *
    FROM orders
    WHERE status = 'pending'
""";
```

### 1.14 Raw Strings (no escape processing)
```
let path = r"C:\Users\kyere\Documents";
let regex = r"\d+\.\d+";
```

### 1.15 Destructuring Assignment
```
// Array destructuring
let [a, b, c] = [1, 2, 3];
let [first, ...rest] = items;
let [x, , z] = coords;           // skip middle

// Object destructuring
let {name, age} = person;
let {name: fullName, age: years} = person;   // rename
let {x = 0, y = 0} = point;                 // with defaults

// In function parameters
function draw({x, y, color = "black"}) { ... }
function sum([a, b, c]) { return a + b + c; }

// Nested
let {address: {city, zip}} = user;
```

### 1.16 Spread / Rest Operators
```
// Spread into array
let combined = [...arr1, ...arr2, 99];

// Spread into object
let merged = {...defaults, ...overrides};

// Rest in function params
function log(level, ...messages) {
    for (m in messages) { print level + ": " + m; }
}

// Rest in destructuring
let [head, ...tail] = list;
```

### 1.17 Default Function Parameters
```
function greet(name, greeting = "Hello") {
    return greeting + ", " + name + "!";
}

function createUser(name, role = "viewer", active = true) { ... }
```

### 1.18 Named / Keyword Arguments
```
function connect(host, port = 5432, ssl = false) { ... }
connect(host: "localhost", ssl: true);    // skip port, name ssl
```

### 1.19 `super` Keyword
```
class Animal {
    function speak() { return "..."; }
    function describe() { return "I am an animal"; }
}

class Dog extends Animal {
    function speak() {
        let parent = super.speak();
        return parent + " woof!";
    }
    function init(name) {
        super.init();      // call parent constructor
        this.name = name;
    }
}
```

### 1.20 Static Methods and Properties
```
class MathHelper {
    static PI = 3.14159265358979;

    static function circle_area(r) {
        return MathHelper.PI * r * r;
    }
}

MathHelper.circle_area(5);
```

### 1.21 Private Fields and Methods
```
class BankAccount {
    private balance = 0;
    private #pin;            // hard-private syntax

    function init(pin) { this.#pin = pin; }

    function deposit(amount) {
        this.#validate(amount);
        this.balance += amount;
    }

    private function #validate(amount) {
        if (amount <= 0) throw "invalid amount";
    }
}
```

### 1.22 Getters and Setters
```
class Temperature {
    function init(celsius) { this._c = celsius; }

    get fahrenheit() { return this._c * 9/5 + 32; }
    set fahrenheit(f) { this._c = (f - 32) * 5/9; }
}

let t = new Temperature(100);
print t.fahrenheit;      // 212
t.fahrenheit = 32;
print t._c;              // 0
```

### 1.23 Computed Property Names
```
let key = "name";
let obj = { [key]: "Alice", ["age"]: 30 };

let methods = {};
methods["get_" + field] = function() { ... };
```

### 1.24 Shorthand Object Properties
```
let name = "Alice";
let age = 30;
let person = {name, age};    // same as {name: name, age: age}
```

### 1.25 Method Shorthand in Objects
```
let obj = {
    greet() { return "hello"; },        // instead of greet: function() { ... }
    get name() { return this._name; }
};
```

---

## Phase 2 — Type System

### 2.1 Optional Type Annotations
```
function add(a: number, b: number) -> number {
    return a + b;
}

let name: string = "Alice";
let scores: array<number> = [95, 87, 92];
```

### 2.2 Enum Types
```
enum Direction { North, South, East, West }
enum Status { Active = 1, Inactive = 0, Pending = -1 }
enum Color { Red = "#FF0000", Green = "#00FF00", Blue = "#0000FF" }

let d = Direction.North;
match d {
    Direction.North => print "going north",
    Direction.South => print "going south",
    _ => print "going sideways"
}
```

### 2.3 Union Types
```
function format(value: string | number) -> string { ... }
let id: string | number = get_id();
```

### 2.4 Tuple Types
```
let point: (number, number) = (3.0, 4.0);
let [x, y] = point;

function min_max(arr: array<number>) -> (number, number) {
    return (first(sort(arr)), last(sort(arr)));
}
let (lo, hi) = min_max([5, 2, 8, 1]);
```

### 2.5 Type Aliases
```
type UserId = number;
type Callback = function(string) -> null;
type Matrix = array<array<number>>;
```

### 2.6 Generics / Parametric Types
```
function identity<T>(x: T) -> T { return x; }
function first_of<T>(arr: array<T>) -> T | null { ... }

class Stack<T> {
    function push(item: T) { ... }
    function pop() -> T | null { ... }
}
```

### 2.7 Interface / Protocol / Trait
```
interface Printable {
    function to_string() -> string;
}

interface Comparable {
    function compare(other) -> number;  // -1, 0, 1
}

class Person implements Printable, Comparable {
    function to_string() { return this.name; }
    function compare(other) { return this.age - other.age; }
}
```

### 2.8 Result / Option Types (Rust-style)
```
function divide(a, b) -> Result<number, string> {
    if (b == 0) return Err("division by zero");
    return Ok(a / b);
}

let result = divide(10, 2);
match result {
    Ok(v)  => print "got: " + v,
    Err(e) => print "error: " + e
}

// Option type
function find_user(id) -> Option<User> {
    // ...
    return Some(user);    // or None
}
```

---

## Phase 3 — Functional Programming Features

### 3.1 Pipe Operator
```
let result = [1,2,3,4,5]
    |> filter(_, function(x) { return x % 2 == 0; })
    |> map(_, function(x) { return x * 10; })
    |> reduce(_, function(acc, x) { return acc + x; }, 0);
```

### 3.2 Partial Application / Currying
```
let add = function(a, b) { return a + b; };
let add5 = partial(add, 5);      // fix first arg
add5(3);   // 8

// Auto-curry with curry()
let curried_add = curry(function(a, b, c) { return a + b + c; });
let step1 = curried_add(1);      // function(b, c)
let step2 = step1(2);            // function(c)
step2(3);                        // 6
```

### 3.3 Function Composition
```
let transform = compose(to_upper, trim, to_string);
transform("  hello  ");   // "HELLO"

let pipeline = pipe(trim, to_lower, function(s) { return s + "!"; });
pipeline("  Hello  ");    // "hello!"
```

### 3.4 Memoization
```
let fib = memoize(function(n) {
    if (n <= 1) return n;
    return fib(n-1) + fib(n-2);
});
fib(50);   // instant, not exponential
```

### 3.5 Generators / Lazy Sequences
```
function* range_gen(start, end) {
    let i = start;
    while (i < end) {
        yield i;
        i += 1;
    }
}

let gen = range_gen(0, 1000000);
gen.next();   // {value: 0, done: false}
gen.next();   // {value: 1, done: false}

// Lazy iteration
for (n in range_gen(0, 10)) {
    print n;
}
```

### 3.6 Pattern Matching Guards (When Clauses)
```
match score {
    n when n >= 90 => "A",
    n when n >= 80 => "B",
    n when n >= 70 => "C",
    n when n >= 60 => "D",
    _              => "F"
}
```

### 3.7 Algebraic Data Types / Tagged Unions
```
type Shape =
    | Circle { radius: number }
    | Rectangle { width: number, height: number }
    | Triangle { base: number, height: number };

function area(shape: Shape) -> number {
    return match shape {
        Circle { radius: r }          => PI * r * r,
        Rectangle { width: w, height: h } => w * h,
        Triangle { base: b, height: h }   => 0.5 * b * h
    };
}
```

### 3.8 Tail Call Optimization
Proper TCO so `function inf() { return inf(); }` doesn't stack overflow — converted to a loop internally.

### 3.9 Immutable Data Helpers
```
let original = {name: "Alice", age: 30};
let updated = update(original, {age: 31});   // new object, original unchanged

let arr = [1, 2, 3];
let new_arr = set_at(arr, 1, 99);            // [1, 99, 3], original unchanged
```

---

## Phase 4 — Standard Library Expansion

### 4.1 Math Module
```
import "std/math";

math.PI         // 3.14159265358979
math.E          // 2.71828182845904
math.TAU        // 6.28318530717959
math.INFINITY   // Infinity
math.NAN        // NaN

math.random()                  // 0.0 to 1.0
math.random_int(1, 6)          // random integer 1-6
math.random_choice([...])      // random element

math.clamp(value, min, max)
math.lerp(a, b, t)             // linear interpolation
math.sign(x)                   // -1, 0, or 1
math.gcd(a, b)
math.lcm(a, b)
math.factorial(n)
math.is_nan(x)
math.is_finite(x)
math.hypot(x, y)               // sqrt(x^2 + y^2)
math.degrees(radians)
math.radians(degrees)
```

### 4.2 String Module
```
// Additional string functions
repeat("ha", 3)              // "hahaha"
pad_start("5", 3, "0")       // "005"
pad_end("5", 3, "-")         // "5--"
at("hello", -1)              // "o"  (negative indexing)
index_of("hello", "l")       // 2
last_index_of("hello", "l")  // 3
char_codes("hi")             // [104, 105]
from_char_codes([72, 105])   // "Hi"
is_digit(c)
is_alpha(c)
is_alphanumeric(c)
is_whitespace(c)
is_upper(c)
is_lower(c)
count_occurrences(str, substr)
reverse_string(s)
word_count(s)
lines(s)                     // split by newlines
```

### 4.3 Array Module
```
flat([1, [2, [3, 4]]])                    // [1, 2, 3, 4]
flat_map([1,2,3], function(x) { return [x, x*2]; })
fill(5, 0)                                // [0, 0, 0, 0, 0]
fill_with(5, function(i) { return i*i; }) // [0, 1, 4, 9, 16]
zip([1,2,3], ["a","b","c"])              // [[1,"a"],[2,"b"],[3,"c"]]
unzip([[1,"a"],[2,"b"]])                 // [[1,2],["a","b"]]
chunk([1,2,3,4,5], 2)                    // [[1,2],[3,4],[5]]
unique([1,2,2,3,3,3])                    // [1,2,3]
group_by(people, function(p) { return p.age; })
partition([1,2,3,4,5], function(x) { return x%2==0; })
                                         // [[2,4], [1,3,5]]
rotate([1,2,3,4,5], 2)                  // [3,4,5,1,2]
take([1,2,3,4,5], 3)                    // [1,2,3]
drop([1,2,3,4,5], 2)                    // [3,4,5]
take_while(arr, predicate)
drop_while(arr, predicate)
flatten_deep(arr)
count(arr, predicate)
sum(arr)
average(arr)
min_by(people, function(p) { return p.age; })
max_by(people, function(p) { return p.age; })
sort_by(people, function(p) { return p.name; })
difference([1,2,3,4], [2,4])            // [1,3]
intersection([1,2,3], [2,3,4])          // [2,3]
union([1,2,3], [3,4,5])                 // [1,2,3,4,5]
```

### 4.4 Object / Map Module
```
merge(obj1, obj2, obj3)              // deep merge
deep_clone(obj)
get_path(obj, "address.city")        // nested get
set_path(obj, "address.city", val)   // nested set
omit(obj, ["password", "pin"])
pick(obj, ["name", "email"])
map_values(obj, function(v) { return v * 2; })
map_keys(obj, function(k) { return to_upper(k); })
filter_values(obj, function(v) { return v != null; })
invert(obj)                          // swap keys and values
from_entries([["a",1],["b",2]])      // {a:1, b:2}
```

### 4.5 Map and Set Data Structures
```
// Ordered Map (any key type, including objects)
let m = new Map();
m.set("key", 42);
m.get("key");       // 42
m.has("key");       // true
m.delete("key");
m.size;             // 0
for ([k, v] in m) { ... }

// Set (unique values)
let s = new Set([1, 2, 3, 2, 1]);
s.add(4);
s.has(2);           // true
s.delete(3);
s.size;             // 3
let arr = s.to_array();
s.union(other_set);
s.intersection(other_set);
s.difference(other_set);
```

### 4.6 Date / Time
```
import "std/datetime";

let now = DateTime.now();
let epoch = DateTime.from_timestamp(1716000000);
let d = DateTime.new(2024, 12, 25);

d.year; d.month; d.day; d.hour; d.minute; d.second;
d.day_of_week;       // 0=Sunday
d.is_leap_year();
d.to_timestamp();
d.to_string("YYYY-MM-DD HH:mm:ss");
d.add_days(7);
d.add_months(1);
d.diff(other_date, "days");

let timer = Timer.start();
// ... do work ...
print timer.elapsed_ms();
```

### 4.7 JSON
```
import "std/json";

let obj = json.parse('{"name":"Alice","age":30}');
let str = json.stringify(obj);
let pretty = json.stringify(obj, indent: 2);
json.is_valid(str);
```

### 4.8 Regular Expressions
```
import "std/regex";

let pattern = regex.new(r"\d+\.\d+");
pattern.test("3.14");              // true
pattern.match("price: 9.99");     // ["9.99"]
pattern.match_all("1.1 and 2.2"); // ["1.1", "2.2"]
regex.replace("hello", "l", "r"); // "herro"

// Literal regex syntax
let re = /\d+/g;
"abc123def456".match_all(re);     // ["123", "456"]
```

### 4.9 File System
```
import "std/fs";

fs.read_text("file.txt")
fs.write_text("file.txt", content)
fs.append_text("file.txt", content)
fs.read_bytes("image.png")
fs.write_bytes("out.bin", bytes)
fs.exists("path/to/file")
fs.delete("file.txt")
fs.rename("old.txt", "new.txt")
fs.copy("src.txt", "dst.txt")
fs.mkdir("new/dir")
fs.mkdir_all("a/b/c/d")
fs.rmdir("dir")
fs.list_dir(".")                   // array of filenames
fs.list_dir_recursive("src")
fs.is_file("path")
fs.is_dir("path")
fs.file_size("file.txt")
fs.last_modified("file.txt")
fs.temp_file()                     // create temp file, return path
fs.temp_dir()
```

### 4.10 Path Manipulation
```
import "std/path";

path.join("src", "utils", "math.cl")     // "src/utils/math.cl"
path.dirname("/home/user/file.txt")      // "/home/user"
path.basename("/home/user/file.txt")     // "file.txt"
path.stem("file.txt")                    // "file"
path.extension("file.txt")              // "txt"
path.absolute("relative/path")
path.normalize("a/b/../c")             // "a/c"
path.split("a/b/c")                    // ["a","b","c"]
path.is_absolute("/usr/bin")           // true
```

### 4.11 Process / System
```
import "std/process";

process.args()          // command-line arguments
process.env("HOME")     // environment variable
process.env_all()       // all environment variables
process.cwd()           // current working directory
process.chdir("/tmp")
process.exit(0)
process.pid()
process.platform()      // "windows", "linux", "macos"

// Execute shell commands
let result = process.run("ls -la");
result.stdout;
result.stderr;
result.exit_code;

let proc = process.spawn("python", ["script.py"]);
proc.write("input data");
proc.read_line();
proc.kill();
```

### 4.12 HTTP Client
```
import "std/http";

let res = http.get("https://api.example.com/users");
res.status;            // 200
res.body;              // string
res.json();            // parsed JSON
res.headers;

let res2 = http.post("https://api.example.com/users", {
    body: json.stringify({name: "Alice"}),
    headers: {"Content-Type": "application/json"}
});

http.put(url, options);
http.delete(url, options);
http.patch(url, options);
```

### 4.13 HTTP Server
```
import "std/http_server";

let server = http_server.new(port: 8080);

server.get("/", function(req, res) {
    res.send("Hello, World!");
});

server.post("/users", function(req, res) {
    let user = json.parse(req.body);
    res.json({id: 1, ...user});
});

server.use(function(req, res, next) {    // middleware
    print req.method + " " + req.path;
    next();
});

server.listen();
print "Server running on port 8080";
```

### 4.14 Random
```
import "std/random";

random.float()                    // 0.0 to 1.0
random.float(min, max)            // min to max
random.int(min, max)              // inclusive
random.bool()
random.choice(array)
random.shuffle(array)             // returns new shuffled array
random.sample(array, n)          // n unique random elements
random.seed(42)                   // reproducible randomness
random.uuid()                     // "550e8400-e29b-41d4-a716-446655440000"
```

### 4.15 Cryptography
```
import "std/crypto";

crypto.sha256("hello")            // hex string
crypto.sha512("hello")
crypto.md5("hello")               // not secure, for checksums
crypto.hmac_sha256(key, message)
crypto.base64_encode(str)
crypto.base64_decode(str)
crypto.hex_encode(bytes)
crypto.hex_decode(hex_str)
crypto.random_bytes(32)           // secure random bytes
crypto.compare_secure(a, b)       // timing-safe comparison
```

### 4.16 Encoding / Decoding
```
import "std/encoding";

encoding.url_encode("hello world")       // "hello%20world"
encoding.url_decode("hello%20world")     // "hello world"
encoding.html_encode("<b>hi</b>")        // "&lt;b&gt;hi&lt;/b&gt;"
encoding.html_decode("&lt;b&gt;")        // "<b>"
encoding.base64_encode("binary data")
encoding.base64_decode("YmluYXJ5IGRhdGE=")
```

### 4.17 Parsing Formats
```
import "std/csv";
import "std/toml";
import "std/yaml";
import "std/ini";
import "std/xml";

let rows = csv.parse("name,age\nAlice,30\n");
let config = toml.parse(read_file("config.toml"));
let data = yaml.parse(read_file("data.yaml"));
```

### 4.18 Collections Library
```
// Queue
let q = new Queue();
q.enqueue(1); q.enqueue(2);
q.dequeue();     // 1
q.peek();        // 2
q.is_empty();

// Stack
let s = new Stack();
s.push(1); s.push(2);
s.pop();         // 2
s.peek();        // 1

// LinkedList
let ll = new LinkedList();
ll.prepend(1); ll.append(2);
ll.to_array();

// PriorityQueue
let pq = new PriorityQueue(function(a,b) { return a.priority - b.priority; });

// Deque (double-ended queue)
let d = new Deque();
d.push_front(1); d.push_back(2);
d.pop_front(); d.pop_back();
```

---

## Phase 5 — Module System Improvements

### 5.1 Named and Default Exports
```
// math_utils.cl
export function add(a, b) { return a + b; }
export function multiply(a, b) { return a * b; }
export default function main_function() { ... }    // default export
export let VERSION = "1.0.0";
```

### 5.2 Selective Imports
```
import { add, multiply } from "math_utils";
import { add as sum } from "math_utils";          // rename
import * as math from "math_utils";               // namespace
import default_fn from "math_utils";              // default export
```

### 5.3 Re-exports
```
// index.cl — barrel file
export { add, multiply } from "math_utils";
export { format_date } from "datetime_utils";
export * from "string_utils";
```

### 5.4 Module Caching
Currently, importing the same module twice re-executes it. Should be cached after first load.

### 5.5 Circular Import Detection
Detect and give a clear error when two modules import each other.

### 5.6 Standard Library Modules
```
import "std/math";
import "std/string";
import "std/array";
import "std/fs";
import "std/path";
import "std/http";
import "std/json";
import "std/regex";
import "std/random";
import "std/crypto";
import "std/datetime";
import "std/process";
import "std/os";
import "std/collections";
import "std/testing";
```

### 5.7 Package Manager (clpm — Custom Language Package Manager)
```
# clpm.toml
[package]
name = "my-project"
version = "1.0.0"

[dependencies]
http-client = "2.1.0"
json-schema = "1.0.0"
```

---

## Phase 6 — Concurrency

### 6.1 Async / Await
```
async function fetch_user(id) {
    let res = await http.get("/users/" + id);
    return json.parse(res.body);
}

let user = await fetch_user(42);

// Parallel
let [users, posts] = await Promise.all([
    fetch_user(1),
    fetch_posts(1)
]);
```

### 6.2 Promises / Futures
```
let p = Promise.new(function(resolve, reject) {
    let result = do_slow_work();
    if (result.ok) resolve(result.value);
    else reject(result.error);
});

p.then(function(v) { print v; })
 .catch(function(e) { print "error: " + e; })
 .finally(function() { cleanup(); });
```

### 6.3 Channels (CSP-style)
```
let ch = new Channel(buffer: 10);

spawn function() {
    for (i in range(5)) {
        ch.send(i);
    }
    ch.close();
};

for (val in ch) {
    print val;
}
```

### 6.4 Actor Model
```
let actor = spawn_actor(function(state, msg) {
    match msg.type {
        "increment" => return state + 1,
        "get" => { msg.reply(state); return state; }
    }
});

actor.send({type: "increment"});
let count = actor.ask({type: "get"});
```

---

## Phase 7 — Developer Tools

### 7.1 Built-in Test Runner
```
import "std/testing";

test("adds two numbers", function() {
    assert_eq(add(2, 3), 5);
    assert_neq(add(2, 3), 6);
});

test("handles errors", function() {
    assert_throws(function() { divide(1, 0); });
});

describe("BankAccount", function() {
    let account;

    before_each(function() {
        account = new BankAccount(1000);
    });

    test("deposits correctly", function() {
        account.deposit(500);
        assert_eq(account.balance, 1500);
    });
});
```
Run with: `custom-lang test`

### 7.2 REPL Improvements
- Multi-line input (detect incomplete statements, show `...` prompt)
- Persistent history across sessions (save to `~/.custom_lang_history`)
- Tab completion (variable names, function names, methods)
- Syntax highlighting in REPL
- `.load file.cl` to load a file into REPL session
- `.reset` to clear all variables
- `.vars` to show all defined variables
- `.type expr` to show the type of an expression
- `.time expr` to time an expression

### 7.3 Formatter
```
custom-lang fmt file.cl          # format in place
custom-lang fmt --check file.cl  # check without modifying
custom-lang fmt src/             # format directory
```

### 7.4 Linter
```
custom-lang lint file.cl
```
Rules:
- Unused variables
- Unreachable code
- Missing return in function
- Division by variable that could be zero
- Infinite loops (simple cases)
- Shadowed variables

### 7.5 Language Server (LSP)
Enables VS Code / Neovim / JetBrains integration:
- Go to definition
- Find references
- Hover type info
- Auto-complete
- Rename symbol
- Inline errors

### 7.6 Documentation Generator
```
/// Adds two numbers together.
/// @param a The first number
/// @param b The second number
/// @returns The sum
function add(a, b) { return a + b; }
```
`custom-lang docs src/ --output docs/`

### 7.7 REPL Notebook Mode
Jupyter-style interactive cells, output persisted alongside code.

### 7.8 Debugger
```
custom-lang debug file.cl
```
- Breakpoints
- Step over / step into / step out
- Watch expressions
- Call stack inspection
- Variable inspection

### 7.9 Profiler
```
custom-lang profile file.cl
```
- Shows time spent per function
- Shows hot paths
- Identifies slow builtins

---

## Phase 8 — Performance and Compilation

### 8.1 Bytecode Compiler + VM
Instead of tree-walking, compile AST → bytecode, run on a register or stack VM.
Rough 10-100× speedup.

### 8.2 JIT Compilation
Compile hot paths to native machine code at runtime.

### 8.3 Ahead-of-Time Compilation
```
custom-lang compile file.cl --output file         # native binary
custom-lang compile file.cl --target wasm         # WebAssembly
custom-lang compile file.cl --target js           # JavaScript
```

### 8.4 Proper Tail Call Optimization
Eliminate stack frames for tail-recursive calls — enable O(1) stack recursion.

### 8.5 True Garbage Collection
Replace `Rc<RefCell<>>` (reference counting, leaks on cycles) with a proper tracing GC.

### 8.6 Interning
Intern strings and small integers for O(1) equality and reduced memory.

---

## Phase 9 — Interoperability

### 9.1 Foreign Function Interface (FFI)
```
// Call C functions from custom-lang
ffi.load("libcurl.so");
let curl = ffi.function("curl_easy_init", returns: "pointer");
let handle = curl();
```

### 9.2 Embedding API
Use the interpreter as a Rust library:
```rust
let mut interp = Interpreter::new();
interp.set_global("config", json_to_value(&config));
interp.eval_file("plugin.cl")?;
let result = interp.call("on_event", &[Value::Str("click".into())])?;
```

### 9.3 WebAssembly Target
Compile custom-lang scripts to WASM so they run in browsers.

### 9.4 JavaScript Interop
When compiled to JS, access browser APIs:
```
import "std/dom";
let el = dom.get_by_id("my-div");
el.set_text("Hello from custom-lang!");
```

### 9.5 Python Interop
Call Python libraries from custom-lang:
```
import "interop/python";
let np = python.import("numpy");
let arr = np.array([1, 2, 3, 4, 5]);
```

---

## Phase 10 — Advanced / Exotic Features

### 10.1 Macros / Metaprogramming
```
macro unless(cond, body) {
    if (!cond) { body }
}

unless (x == 0) {
    print "x is not zero";
}
```

### 10.2 Decorators / Annotations
```
@memoize
function fib(n) { ... }

@deprecated("Use new_function instead")
function old_function() { ... }

@validate(schema: UserSchema)
function create_user(data) { ... }
```

### 10.3 Operator Overloading
```
class Vector {
    function init(x, y) { this.x = x; this.y = y; }

    operator +(other) { return new Vector(this.x + other.x, this.y + other.y); }
    operator *(scalar) { return new Vector(this.x * scalar, this.y * scalar); }
    operator ==(other) { return this.x == other.x && this.y == other.y; }
    operator to_string() { return "(" + this.x + ", " + this.y + ")"; }
}

let v = new Vector(1, 2) + new Vector(3, 4);   // Vector(4, 6)
```

### 10.4 Custom Iterable Protocol
Make any object work with `for (x in obj)`:
```
class Range {
    function init(start, end) { this.start = start; this.end = end; }

    function [Symbol.iterator]() {
        let i = this.start;
        let end = this.end;
        return {
            next: function() {
                if (i < end) { let v = i; i += 1; return {value: v, done: false}; }
                return {value: null, done: true};
            }
        };
    }
}

for (n in new Range(1, 5)) { print n; }
```

### 10.5 Proxies / Reflection
```
let proxy = new Proxy(target, {
    get: function(obj, key) { print "getting " + key; return obj[key]; },
    set: function(obj, key, val) { print "setting " + key; obj[key] = val; }
});
```

### 10.6 Continuations / Call-CC
```
let saved;

let result = call_cc(function(k) {
    saved = k;       // save continuation
    return 1;
});

// ... later
saved(42);           // jump back, result = 42
```

### 10.7 Coroutines
```
function* producer() {
    let i = 0;
    while (true) {
        yield i;
        i += 1;
    }
}

let gen = producer();
gen.next().value;    // 0
gen.next().value;    // 1
gen.next().value;    // 2
```

### 10.8 Gradual / Optional Static Typing
Opt-in type checking: unannotated code runs as-is, annotated code is checked.
```
// strict mode for this file:
#use strict types

function greet(name: string) -> string {
    return "Hello, " + name;
}

greet(42);   // type error at compile time
```

### 10.9 Sandboxed Execution
```
let sandbox = Sandbox.new({
    allow_fs: false,
    allow_network: false,
    allow_process: false,
    memory_limit_mb: 64,
    timeout_ms: 5000
});

sandbox.eval(untrusted_code);
```

### 10.10 Hot Module Reloading
For long-running programs, reload changed modules without restart:
```
import "std/watcher";

watcher.watch("config.cl", function() {
    reimport "config";
    print "Config reloaded!";
});
```

### 10.11 WASI Support
Run custom-lang scripts on any platform via WebAssembly System Interface.

### 10.12 Persistent Variables Across REPL Sessions
```
// In REPL:
@persist let my_data = load_big_dataset();
// Close and reopen REPL — my_data is still there
```

### 10.13 Multi-line Lambda Shorthand
```
// Instead of:
let f = function(x) { return x * 2; };

// Short form:
let f = x => x * 2;
let g = (x, y) => x + y;
let h = x => {
    let result = x * x;
    return result + 1;
};
```

### 10.14 String Padding in Print (Format Strings)
```
print format("{:>10}", name);           // right-align in 10 chars
print format("{:.2f}", 3.14159);        // 2 decimal places
print format("{:05d}", 42);             // "00042"
print format("{:x}", 255);             // "ff" (hex)
print format("{:b}", 10);              // "1010" (binary)
```

### 10.15 Weak References (for caches, etc.)
```
import "std/weak";

let cache = new WeakMap();    // doesn't prevent garbage collection of keys
let obj = {};
cache.set(obj, compute_expensive(obj));
// When obj goes out of scope, cache entry is automatically removed
```

---

## Summary Checklist

| Category | Count | Priority |
|---|---|---|
| Core syntax gaps | 25 features | 🔴 High |
| Type system | 8 features | 🟡 Medium |
| Functional programming | 9 features | 🟡 Medium |
| Standard library | 18 modules | 🔴 High |
| Module system | 7 improvements | 🟡 Medium |
| Concurrency | 4 models | 🟠 Medium-High |
| Developer tools | 9 tools | 🟡 Medium |
| Performance/compilation | 6 items | 🟢 Long-term |
| Interoperability | 5 targets | 🟢 Long-term |
| Advanced/exotic | 15 features | 🟢 Long-term |

**Start here when resuming:**
1. `try/catch/throw` — most impactful single feature
2. `super` keyword — OOP is incomplete without it
3. Default parameters + destructuring + spread — used in every real program
4. Ternary operator — everyone expects it
5. `std/json` and `std/random` — needed for 90% of real scripts
6. Multi-line REPL input — the REPL is hard to use without it
