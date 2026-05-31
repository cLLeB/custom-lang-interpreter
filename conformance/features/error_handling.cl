// try/catch captures a thrown value; throw propagates until caught.
try {
  throw "boom"
} catch (e) {
  print("caught: " + e)
}

function risky(x) {
  if (x < 0) {
    throw "negative input"
  }
  return x * 2
}

try {
  print("ok: " + risky(5))
  print("ok: " + risky(-1))
} catch (e) {
  print("error: " + e)
}
print("after")
