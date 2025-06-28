// === File I/O Demo ===
// File Operations in Custom Language

// 1. Writing to files
print "=== File I/O Demo ===";
print "Writing data to files...";
print "";

// Write simple text
let success1 = write_file("test_output.txt", "Hello, Custom Language!");
print "Write simple text: " + success1;

// Write numbers and calculations
let result = 42 * 3 + 15;
let success2 = write_file("calculations.txt", "The answer is: " + result);
print "Write calculation result: " + success2;

// Write array data
let data = [1, 2, 3, 4, 5];
let success3 = write_file("array_data.txt", "Array: " + data);
print "Write array data: " + success3;

// Write formatted data
let name = "Custom Language";
let version = "0.2.0";
let info = "Language: " + name + "\nVersion: " + version + "\nFeatures: File I/O, Arrays, Functions";
let success4 = write_file("info.txt", info);
print "Write formatted info: " + success4;

print "";

// 2. Reading from files
print "Reading data from files...";
print "";

// Read the files we just created
let content1 = read_file("test_output.txt");
print "Content of test_output.txt: " + content1;

let content2 = read_file("calculations.txt");
print "Content of calculations.txt: " + content2;

let content3 = read_file("array_data.txt");
print "Content of array_data.txt: " + content3;

let content4 = read_file("info.txt");
print "Content of info.txt:";
print content4;

print "";

// 3. File processing example
print "File Processing Example...";
print "";

// Create a data file with numbers
let numbers_text = "10\n20\n30\n40\n50";
write_file("numbers.txt", numbers_text);
print "Created numbers.txt with data";

// Read and display the content
let numbers_content = read_file("numbers.txt");
print "Numbers file content:";
print numbers_content;

print "";

// 4. Configuration file example
print "Configuration File Example...";
print "";

// Create a simple config file
let config = "# Custom Language Configuration\n";
config = config + "debug_mode=true\n";
config = config + "max_iterations=1000\n";
config = config + "output_format=json\n";
config = config + "# End of configuration";

write_file("config.txt", config);
print "Created configuration file";

let config_content = read_file("config.txt");
print "Configuration content:";
print config_content;

print "";

// 5. Error handling demonstration
print "Error Handling Demo...";
print "";

// This will demonstrate error handling for non-existent files
print "Attempting to read non-existent file...";
// Uncomment the next line to see error handling in action:
// let missing = read_file("does_not_exist.txt");

print "";

// 6. Practical example: Simple log file
print "Practical Example: Log File...";
print "";

function log_message(message) {
    let timestamp = "2024-01-01 12:00:00"; // Simplified timestamp
    let log_entry = "[" + timestamp + "] " + message + "\n";
    
    // In a real implementation, we'd append to the file
    // For now, we'll create/overwrite
    write_file("app.log", log_entry);
    return true;
}

// Log some messages
log_message("Application started");
log_message("User logged in");
log_message("Processing data...");
log_message("Operation completed successfully");

let log_content = read_file("app.log");
print "Application log:";
print log_content;

print "";

print "=== File I/O Demo Complete! ===";
print "";
print "New File I/O Features:";
print "- read_file(filename) - Read entire file content as string";
print "- write_file(filename, content) - Write content to file";
print "- Automatic type conversion for content";
print "- Comprehensive error handling";
print "- Support for text files, data files, and configuration files";
print "";
print "Use Cases:";
print "- Data persistence between program runs";
print "- Configuration file management";
print "- Log file creation and reading";
print "- Data import/export functionality";
print "- Simple database-like operations";
print "";
print "Next Steps:";
print "- Try creating your own files";
print "- Experiment with different data formats";
print "- Build file-based applications";
print "- Create configuration systems";
