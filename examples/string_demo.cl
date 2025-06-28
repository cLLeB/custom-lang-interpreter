// === String Manipulation Demo ===
// Comprehensive demonstration of string functions

print "=== String Manipulation Demo ===";
print "";

// 1. Basic string operations
print "1. Basic String Operations:";
let text = "Hello, Custom Language!";
print "Original text: " + text;
print "Length: " + len(text);
print "Uppercase: " + to_upper(text);
print "Lowercase: " + to_lower(text);
print "";

// 2. String trimming
print "2. String Trimming:";
let messy_text = "   Hello World   ";
print "Original: '" + messy_text + "'";
print "Trimmed: '" + trim(messy_text) + "'";
print "";

// 3. Substring operations
print "3. Substring Operations:";
let sample = "Programming";
print "Original: " + sample;
print "substring(0, 4): " + substring(sample, 0, 4);
print "substring(4): " + substring(sample, 4);
print "substring(0, 7): " + substring(sample, 0, 7);
print "";

// 4. String splitting and joining
print "4. String Splitting and Joining:";
let sentence = "apple,banana,cherry,date";
print "Original: " + sentence;
let fruits = split(sentence, ",");
print "Split by comma: " + fruits;
print "Array length: " + len(fruits);
print "First fruit: " + fruits[0];
print "Last fruit: " + fruits[3];

// Join them back with different delimiter
let rejoined = join(fruits, " | ");
print "Rejoined with ' | ': " + rejoined;
print "";

// 5. String searching
print "5. String Searching:";
let phrase = "The quick brown fox jumps over the lazy dog";
print "Text: " + phrase;
print "Contains 'fox': " + contains(phrase, "fox");
print "Contains 'cat': " + contains(phrase, "cat");
print "Starts with 'The': " + starts_with(phrase, "The");
print "Starts with 'A': " + starts_with(phrase, "A");
print "Ends with 'dog': " + ends_with(phrase, "dog");
print "Ends with 'cat': " + ends_with(phrase, "cat");
print "";

// 6. String replacement
print "6. String Replacement:";
let original = "I love programming in Python";
print "Original: " + original;
let replaced = replace(original, "Python", "Custom Language");
print "Replaced: " + replaced;

let multiple_replace = replace("Hello Hello Hello", "Hello", "Hi");
print "Multiple replace: " + multiple_replace;
print "";

// 7. Practical examples
print "7. Practical Examples:";
print "";

// Email validation (simple)
function is_valid_email(email) {
    return contains(email, "@") && contains(email, ".");
}

let email1 = "user@example.com";
let email2 = "invalid-email";
print "Email validation:";
print email1 + " is valid: " + is_valid_email(email1);
print email2 + " is valid: " + is_valid_email(email2);
print "";

// Name formatting
function format_name(first, last) {
    let formatted_first = to_upper(substring(first, 0, 1)) + to_lower(substring(first, 1));
    let formatted_last = to_upper(substring(last, 0, 1)) + to_lower(substring(last, 1));
    return formatted_first + " " + formatted_last;
}

print "Name formatting:";
print format_name("john", "DOE");
print format_name("JANE", "smith");
print "";

// CSV parsing simulation
function parse_csv_line(line) {
    return split(line, ",");
}

let csv_data = "John,25,Engineer,New York";
let parsed = parse_csv_line(csv_data);
print "CSV parsing:";
print "Raw data: " + csv_data;
print "Parsed: " + parsed;
print "Name: " + parsed[0];
print "Age: " + parsed[1];
print "Job: " + parsed[2];
print "City: " + parsed[3];
print "";

// Text processing
function count_words(text) {
    let words = split(trim(text), " ");
    return len(words);
}

let article = "This is a sample article with multiple words for testing";
print "Text analysis:";
print "Text: " + article;
print "Word count: " + count_words(article);
print "";

// URL parsing (simple)
function extract_domain(url) {
    if (starts_with(url, "http://")) {
        url = substring(url, 7);
    }
    if (starts_with(url, "https://")) {
        url = substring(url, 8);
    }
    let parts = split(url, "/");
    return parts[0];
}

let url1 = "https://www.example.com/path/to/page";
let url2 = "http://github.com/user/repo";
print "URL domain extraction:";
print url1 + " -> " + extract_domain(url1);
print url2 + " -> " + extract_domain(url2);
print "";

print "=== String Demo Complete! ===";
print "";
print "New String Functions:";
print "- split(text, delimiter) - Split string into array";
print "- join(array, delimiter) - Join array into string";
print "- substring(text, start, end?) - Extract substring";
print "- to_upper(text) - Convert to uppercase";
print "- to_lower(text) - Convert to lowercase";
print "- trim(text) - Remove leading/trailing whitespace";
print "- starts_with(text, prefix) - Check if starts with prefix";
print "- ends_with(text, suffix) - Check if ends with suffix";
print "- contains(text, substring) - Check if contains substring";
print "- replace(text, from, to) - Replace all occurrences";
print "";
print "Use Cases:";
print "- Text processing and parsing";
print "- Data validation and formatting";
print "- CSV/JSON-like data handling";
print "- URL and email processing";
print "- User input sanitization";
