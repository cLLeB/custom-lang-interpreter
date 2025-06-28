// Simple File I/O test
print "Testing File I/O functions...";

// Test write_file
let result = write_file("test.txt", "Hello World!");
print "Write result: " + result;

// Test read_file
let content = read_file("test.txt");
print "Read content: " + content;

print "File I/O test complete!";
