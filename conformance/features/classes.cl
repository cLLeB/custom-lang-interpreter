// args: --no-semantic
// Classes: declaration with methods, and instantiation with `new`.
class Greeter {
  function hello() {
    print("hello from Greeter")
  }
  function describe() {
    print("I am a Greeter instance")
  }
}
let g = new Greeter()
g.hello()
g.describe()
