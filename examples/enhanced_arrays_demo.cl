// === Enhanced Array Operations Demo ===
// Demonstration of advanced array manipulation functions

print "=== Enhanced Array Operations Demo ===";
print "";

// 1. Array sorting
print "1. Array Sorting:";
let numbers = [5, 2, 8, 1, 9, 3];
print "Original numbers: " + numbers;
let sorted_numbers = sort(numbers);
print "Sorted numbers: " + sorted_numbers;

let words = ["zebra", "apple", "banana", "cherry"];
print "Original words: " + words;
let sorted_words = sort(words);
print "Sorted words: " + sorted_words;

let mixed = [true, false, true];
print "Original booleans: " + mixed;
let sorted_mixed = sort(mixed);
print "Sorted booleans: " + sorted_mixed;
print "";

// 2. Array reversing
print "2. Array Reversing:";
let sequence = [1, 2, 3, 4, 5];
print "Original sequence: " + sequence;
let reversed = reverse(sequence);
print "Reversed sequence: " + reversed;

let alphabet = ["a", "b", "c", "d"];
print "Original alphabet: " + alphabet;
let reversed_alphabet = reverse(alphabet);
print "Reversed alphabet: " + reversed_alphabet;
print "";

// 3. Array searching
print "3. Array Searching:";
let fruits = ["apple", "banana", "cherry", "date"];
print "Fruits array: " + fruits;
print "Includes 'banana': " + includes(fruits, "banana");
print "Includes 'grape': " + includes(fruits, "grape");

let scores = [85, 92, 78, 96, 88];
print "Scores array: " + scores;
print "Includes 92: " + includes(scores, 92);
print "Includes 100: " + includes(scores, 100);
print "";

// 4. Finding elements
print "4. Finding Elements:";
print "Find 'cherry' in fruits: " + find(fruits, "cherry");
print "Find 'grape' in fruits: " + find(fruits, "grape");
print "Find 96 in scores: " + find(scores, 96);
print "Find 100 in scores: " + find(scores, 100);
print "";

// 5. Practical examples
print "5. Practical Examples:";
print "";

// Student grades management
let student_grades = [78, 85, 92, 67, 88, 95, 73];
print "Student Grades Management:";
print "Original grades: " + student_grades;
let sorted_grades = sort(student_grades);
print "Sorted grades: " + sorted_grades;
print "Highest grade: " + last(sorted_grades);
print "Lowest grade: " + first(sorted_grades);
print "Has perfect score (100): " + includes(student_grades, 100);
print "Has failing grade (below 70): " + includes(student_grades, 67);
print "";

// Inventory management
let product_ids = [1001, 1005, 1002, 1008, 1003];
print "Inventory Management:";
print "Product IDs: " + product_ids;
let sorted_ids = sort(product_ids);
print "Sorted IDs: " + sorted_ids;
print "Looking for product 1005: " + find(product_ids, 1005);
print "Looking for product 1010: " + find(product_ids, 1010);
print "";

// Text processing
let words_list = ["hello", "world", "programming", "language", "custom"];
print "Text Processing:";
print "Words: " + words_list;
let alphabetical = sort(words_list);
print "Alphabetical order: " + alphabetical;
let reverse_alpha = reverse(alphabetical);
print "Reverse alphabetical: " + reverse_alpha;
print "Contains 'programming': " + includes(words_list, "programming");
print "Contains 'python': " + includes(words_list, "python");
print "";

// Data analysis simulation
function analyze_array(arr) {
    let sorted_arr = sort(arr);
    let length = len(arr);
    let min_val = first(sorted_arr);
    let max_val = last(sorted_arr);
    
    print "Array Analysis:";
    print "  Original: " + arr;
    print "  Sorted: " + sorted_arr;
    print "  Length: " + length;
    print "  Min: " + min_val;
    print "  Max: " + max_val;
    print "  Reversed: " + reverse(arr);
}

print "6. Data Analysis Function:";
let dataset = [23, 45, 12, 67, 34, 89, 56];
analyze_array(dataset);
print "";

// Array manipulation chains
print "7. Array Manipulation Chains:";
let original = [3, 1, 4, 1, 5, 9, 2, 6];
print "Original: " + original;
let step1 = sort(original);
print "Step 1 - Sorted: " + step1;
let step2 = reverse(step1);
print "Step 2 - Reversed (desc): " + step2;
let step3 = push(step2, 0);
print "Step 3 - Added 0: " + step3;
print "Final length: " + len(step3);
print "";

// Boolean array operations
print "8. Boolean Array Operations:";
let flags = [true, false, true, true, false];
print "Boolean flags: " + flags;
let sorted_flags = sort(flags);
print "Sorted flags: " + sorted_flags;
print "Has true flag: " + includes(flags, true);
print "Has false flag: " + includes(flags, false);
print "Find first true: " + find(flags, true);
print "";

print "=== Enhanced Arrays Demo Complete! ===";
print "";
print "New Array Functions:";
print "- sort(array) - Sort array elements";
print "- reverse(array) - Reverse array order";
print "- includes(array, value) - Check if array contains value";
print "- find(array, value) - Find first matching element";
print "";
print "Benefits:";
print "- Efficient data organization";
print "- Easy searching and filtering";
print "- Data analysis capabilities";
print "- Functional programming patterns";
print "";
print "Coming Soon:";
print "- filter() with custom predicates";
print "- map() for transformations";
print "- reduce() for aggregations";
print "- Advanced sorting with custom comparators";
