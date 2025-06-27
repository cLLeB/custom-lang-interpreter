// Test file with intentional errors
print "This line is fine";

let x = 10;
let y = 20;

// This will cause a parse error - invalid syntax
let z = x + + y;

print z;

// This will cause a runtime error - undefined variable
print undefined_var;
