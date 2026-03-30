# Rust Programming Language

Rust is a systems programming language focused on **safety**, **concurrency**, and **performance**.

## History

Rust was originally designed by Graydon Hoare at Mozilla Research, with contributions from many others. The compiler is [open source](https://github.com/rust-lang/rust).

## Key Features

- Memory safety without garbage collection
- Concurrency without data races
- Zero-cost abstractions
- Minimal runtime

### Ownership System

Rust's **ownership system** is its most distinctive feature. Each value has a single owner, and the value is dropped when the owner goes out of scope.

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1; // s1 is moved to s2
    // println!("{}", s1); // ERROR: s1 no longer valid
    println!("{}", s2);
}
```

### Borrowing

You can create *references* to values without taking ownership:

1. Shared references (`&T`) — read-only, multiple allowed
2. Mutable references (`&mut T`) — read-write, only one allowed

## Comparison

| Feature | Rust | C++ | Go |
| --- | --- | --- | --- |
| Memory safety | Compile-time | Manual | GC |
| Concurrency | Ownership | Manual | Goroutines |
| Performance | Native | Native | Near-native |

## Conclusion

> Rust empowers developers to build reliable and efficient software.

---

*Learn more at [rust-lang.org](https://www.rust-lang.org).*
