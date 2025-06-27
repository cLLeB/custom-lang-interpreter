// Number Guessing Game
print "=== Number Guessing Game ===";
print "";

// Game configuration
let secret_number = 42;  // In a real game, this would be random
let max_attempts = 5;
let current_attempt = 1;

// Game state
let game_won = false;
let game_over = false;

// Utility functions
function abs_diff(a, b) {
    if (a > b) {
        return a - b;
    } else {
        return b - a;
    }
}

function give_hint(guess, secret) {
    let diff = abs_diff(guess, secret);
    
    if (diff == 0) {
        return "Perfect! You got it!";
    } else if (diff <= 2) {
        return "Very close! You're burning hot!";
    } else if (diff <= 5) {
        return "Close! You're getting warm!";
    } else if (diff <= 10) {
        return "Getting warmer...";
    } else if (diff <= 20) {
        return "You're cold...";
    } else {
        return "You're freezing cold!";
    }
}

function check_guess(guess, secret, attempt) {
    print "Attempt " + attempt + ": You guessed " + guess;
    
    if (guess == secret) {
        print "🎉 Congratulations! You guessed the number!";
        print "It took you " + attempt + " attempts.";
        return true;
    } else {
        if (guess < secret) {
            print "📈 Too low! Try a higher number.";
        } else {
            print "📉 Too high! Try a lower number.";
        }
        
        print "💡 Hint: " + give_hint(guess, secret);
        return false;
    }
}

// Game introduction
print "🎯 Welcome to the Number Guessing Game!";
print "I'm thinking of a number between 1 and 100.";
print "You have " + max_attempts + " attempts to guess it.";
print "Good luck!";
print "";

// Simulate game with predefined guesses
// In a real game, these would be user inputs
let guesses = 25;  // First guess
let guess2 = 50;   // Second guess  
let guess3 = 35;   // Third guess
let guess4 = 40;   // Fourth guess
let guess5 = 42;   // Final guess

// Game loop simulation
print "🎮 Starting the game...";
print "";

// Attempt 1
if (!game_over) {
    game_won = check_guess(guesses, secret_number, current_attempt);
    current_attempt = current_attempt + 1;
    if (game_won) {
        game_over = true;
    }
    print "";
}

// Attempt 2
if (!game_over && current_attempt <= max_attempts) {
    game_won = check_guess(guess2, secret_number, current_attempt);
    current_attempt = current_attempt + 1;
    if (game_won) {
        game_over = true;
    }
    print "";
}

// Attempt 3
if (!game_over && current_attempt <= max_attempts) {
    game_won = check_guess(guess3, secret_number, current_attempt);
    current_attempt = current_attempt + 1;
    if (game_won) {
        game_over = true;
    }
    print "";
}

// Attempt 4
if (!game_over && current_attempt <= max_attempts) {
    game_won = check_guess(guess4, secret_number, current_attempt);
    current_attempt = current_attempt + 1;
    if (game_won) {
        game_over = true;
    }
    print "";
}

// Attempt 5
if (!game_over && current_attempt <= max_attempts) {
    game_won = check_guess(guess5, secret_number, current_attempt);
    current_attempt = current_attempt + 1;
    if (game_won) {
        game_over = true;
    }
    print "";
}

// Game conclusion
if (!game_won) {
    print "💔 Game Over! You've used all " + max_attempts + " attempts.";
    print "The secret number was: " + secret_number;
    print "Better luck next time!";
} else {
    print "🏆 Victory! You're a number guessing champion!";
    
    if (current_attempt <= 2) {
        print "🌟 Amazing! You got it in just " + (current_attempt - 1) + " attempts!";
    } else if (current_attempt <= 4) {
        print "👍 Great job! You solved it efficiently!";
    } else {
        print "✅ Well done! You persevered and won!";
    }
}

print "";
print "📊 Game Statistics:";
print "Secret number: " + secret_number;
print "Total attempts used: " + (current_attempt - 1);
print "Attempts remaining: " + (max_attempts - (current_attempt - 1));

// Bonus: Number analysis
print "";
print "🔍 Number Analysis of " + secret_number + ":";
print "Is even? " + (secret_number % 2 == 0);
print "Is divisible by 3? " + (secret_number % 3 == 0);
print "Is divisible by 7? " + (secret_number % 7 == 0);
print "Square root: " + sqrt(secret_number);

print "";
print "Thanks for playing! 🎮";
