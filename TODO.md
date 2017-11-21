Compiler TODO
-------------

- [ ] Error Handling
  - [ ] Fix all error handling unimplemented!() and TODOs
  - [ ] Check for return in functions that have return type
  - [ ] Type checking
- [ ] Constants
- [ ] Arrays and pointers
- [ ] String constants
- [ ] 16-bit numbers
- [ ] Multiply and divide
- [ ] Optimization
  - [ ] Constant evaluation for binary operators in IR
  - [ ] For functions with 0 frame size, don't modify the stack pointer
  - [ ] For comparisons in a condition, generate smarter code
  - [ ] Use Y register in loops somehow
  - [ ] CLC + ADC #1 -> INC
  - [ ] SEC + SBC #1 -> DEC
- [ ] CLI for compiling arbitrary files
