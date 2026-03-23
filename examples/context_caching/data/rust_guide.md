# Rust Programming Quick Guide

## Ownership and Borrowing

1. Every value has a single owner.
2. Ownership moves on assignment or when passing values into functions.
3. Values are dropped when the owner goes out of scope. Borrowing lets you access data without taking ownership:
   multiple immutable references are allowed, or one mutable reference by itself. Lifetimes make sure references stay
   valid.

## Error Handling

- Use `Result<T, E>` for recoverable errors and propagate with `?`.
- Reserve `panic!` for truly unrecoverable states.
- Prefer custom error types that implement `std::error::Error` to keep context clear.

## Scalar Types

Integers (`i8` through `i128` and `isize`, plus unsigned variants), floating points (`f32`, `f64`), `bool`, and `char`
(a Unicode scalar value). Choose the smallest type that fits your domain and enable overflow checks in debug builds.

## Concurrency and Safety

Rust prevents data races at compile time: values must be `Send` to move across threads and `Sync` to share references.
Use `Arc<T>` for shared ownership, `Mutex<T>` or `RwLock<T>` for interior mutability, and channels for message passing.
The borrow checker enforces that mutable access is exclusive, even in async code.
