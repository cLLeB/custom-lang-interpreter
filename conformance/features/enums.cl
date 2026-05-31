// args: --no-semantic
// Enums: a variant prints as its name and exposes .name / .ordinal.
enum Color { Red, Green, Blue }
print(Color.Red)
print(Color.Green.name)
print(Color.Blue.ordinal)
