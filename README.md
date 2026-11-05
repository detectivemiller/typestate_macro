An exercise in Rust's macros. It generated a Typestate pattern for a struct with the following features:
- intuitive automatically generated abbreviations for states
- support for overlapping names between spaces (spaces meaning a set of states)
- support for custom abbreviations
- transition helper functions that also support arbitrary changes to the original struct
- states wrapped in `Rc<>`

Though it was mainly an exercise I plan on adding a few more features most notably support for sharing a state (struct) between spaces.
This would allow changing the struct through the interface of many spaces.
