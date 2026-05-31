// if/else, while, C-style for (with break + continue), and do-while.
// vm: run
if (3 > 2) {
  print("if-then")
} else {
  print("if-else")
}

let i = 0
let sum = 0
while (i < 5) {
  sum = sum + i
  i = i + 1
}
print("while sum: " + sum)

let acc = 0
for (let j = 0; j < 10; j = j + 1) {
  if (j == 3) { continue }
  if (j == 6) { break }
  acc = acc + j
}
print("for acc: " + acc)

let k = 0
do {
  k = k + 1
} while (k < 3)
print("do-while k: " + k)
