// Enhanced Error Handling and Diagnostics Demo
print "=== Enhanced Error Handling Demo ===";
print "";

print "This demo shows improved error messages with suggestions.";
print "Note: This file contains intentional errors for demonstration.";
print "";

// This should trigger an undefined variable error with suggestion
let name = "John";
let age = 25;

print "Defined variables: name, age";
print "Now trying to access 'nam' (typo for 'name')...";

// Uncomment the line below to see the improved error message:
// print nam; // Typo: should be 'name'

print "";
print "=== Error Handling Features ===";
print "- Smart variable name suggestions using fuzzy matching";
print "- Context-aware error messages with helpful hints";
print "- Colored output with source code context";
print "- Type mismatch guidance with specific suggestions";
print "- Detailed error context with line numbers";
print "";
print "To see error messages in action, uncomment the error line and run again.";
