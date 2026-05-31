// Closures capture their defining environment and keep mutable state.
function make_counter() {
  let count = 0
  return () => {
    count = count + 1
    return count
  }
}
let next = make_counter()
print(next())
print(next())
print(next())
let other = make_counter()
print(other())
